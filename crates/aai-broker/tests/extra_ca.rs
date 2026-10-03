// SPDX-License-Identifier: Apache-2.0
//! Private-CA trust for the broker HTTP client used for OIDC discovery and JWKS.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use aai_broker::upstream::build_http_client;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

fn install_ring() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

struct Material {
    ca_pem: PathBuf,
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
}

fn openssl(dir: &Path, args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("openssl");
    assert!(status.success(), "openssl {args:?}");
}

fn ca_and_localhost(dir: &Path, name: &str) -> Material {
    let ca_cnf = format!(
        "[req]\ndistinguished_name = dn\nx509_extensions = ext\nprompt = no\n[dn]\nCN = {name}\n[ext]\nbasicConstraints = critical,CA:TRUE\nkeyUsage = critical,keyCertSign,cRLSign\n"
    );
    let server_cnf = "[req]\ndistinguished_name = dn\nreq_extensions = ext\nprompt = no\n[dn]\nCN = localhost\n[ext]\nsubjectAltName = IP:127.0.0.1,DNS:localhost\nbasicConstraints = CA:FALSE\nextendedKeyUsage = serverAuth\n";
    std::fs::write(dir.join(format!("{name}-ca.cnf")), ca_cnf).unwrap();
    std::fs::write(dir.join(format!("{name}-server.cnf")), server_cnf).unwrap();
    openssl(
        dir,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            &format!("{name}-ca.key"),
            "-out",
            &format!("{name}-ca.pem"),
            "-days",
            "2",
            "-nodes",
            "-config",
            &format!("{name}-ca.cnf"),
        ],
    );
    openssl(
        dir,
        &[
            "req",
            "-newkey",
            "rsa:2048",
            "-keyout",
            &format!("{name}-server.key"),
            "-out",
            &format!("{name}-server.csr"),
            "-nodes",
            "-config",
            &format!("{name}-server.cnf"),
        ],
    );
    openssl(
        dir,
        &[
            "x509",
            "-req",
            "-in",
            &format!("{name}-server.csr"),
            "-CA",
            &format!("{name}-ca.pem"),
            "-CAkey",
            &format!("{name}-ca.key"),
            "-CAcreateserial",
            "-out",
            &format!("{name}-server.pem"),
            "-days",
            "2",
            "-extfile",
            &format!("{name}-server.cnf"),
            "-extensions",
            "ext",
        ],
    );
    openssl(
        dir,
        &[
            "x509",
            "-in",
            &format!("{name}-server.pem"),
            "-outform",
            "DER",
            "-out",
            &format!("{name}-server.der"),
        ],
    );
    openssl(
        dir,
        &[
            "pkcs8",
            "-topk8",
            "-nocrypt",
            "-in",
            &format!("{name}-server.key"),
            "-outform",
            "DER",
            "-out",
            &format!("{name}-server.key.der"),
        ],
    );
    let cert = CertificateDer::from(std::fs::read(dir.join(format!("{name}-server.der"))).unwrap());
    let key =
        PrivateKeyDer::try_from(std::fs::read(dir.join(format!("{name}-server.key.der"))).unwrap())
            .expect("pkcs8 key");
    Material {
        ca_pem: dir.join(format!("{name}-ca.pem")),
        cert,
        key,
    }
}

async fn serve_ok(listener: TcpListener, acceptor: TlsAcceptor) {
    loop {
        let Ok((sock, _)) = listener.accept().await else {
            break;
        };
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let Ok(mut stream) = acceptor.accept(sock).await else {
                return;
            };
            let mut buf = [0_u8; 2048];
            let _ = stream.read(&mut buf).await;
            let body = b"{\"keys\":[]}";
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes()).await;
            let _ = stream.write_all(body).await;
            let _ = stream.shutdown().await;
        });
    }
}

#[tokio::test]
async fn extra_ca_is_trusted_and_default_roots_are_not() {
    install_ring();
    let dir = tempfile::tempdir().unwrap();
    let signed = ca_and_localhost(dir.path(), "signed");
    let other = ca_and_localhost(dir.path(), "other");

    let server = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![signed.cert], signed.key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(server));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(serve_ok(listener, acceptor));

    let url = format!("https://127.0.0.1:{port}/.well-known/jwks.json");
    let trusted = build_http_client(Some(&signed.ca_pem)).unwrap();
    let body = trusted
        .get(&url)
        .send()
        .await
        .expect("private CA should verify")
        .text()
        .await
        .unwrap();
    assert!(body.contains("keys"));

    let default_client = build_http_client(None).unwrap();
    assert!(
        default_client.get(&url).send().await.is_err(),
        "webpki roots must not trust this private CA"
    );

    let wrong = build_http_client(Some(&other.ca_pem)).unwrap();
    assert!(wrong.get(&url).send().await.is_err());
}

#[test]
fn missing_or_empty_bundle_fails_closed() {
    let missing = build_http_client(Some(std::path::Path::new(
        "/tmp/ga4gh-infra-missing-ca.pem",
    )));
    let text = missing.expect_err("missing bundle").to_string();
    assert!(text.contains("tls.extra_ca_bundle"), "{text}");

    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.pem");
    std::fs::write(&empty, b"not a certificate\n").unwrap();
    let text = build_http_client(Some(&empty))
        .expect_err("empty bundle")
        .to_string();
    assert!(text.contains("no CERTIFICATE"), "{text}");
}
