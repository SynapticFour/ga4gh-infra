// SPDX-License-Identifier: Apache-2.0

//! Data Use Ontology (DUO) term codes.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A Data Use Ontology permission or modifier code.
///
/// Codes correspond to the `oboInOwl:shorthand` values in the
/// [DUO OWL source](https://github.com/EBISPOT/DUO).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum DuoCode {
    /// No restriction on use (`DUO:0000004`, NRES).
    Nres,
    /// Health or medical or biomedical research (`DUO:0000006`, HMB).
    Hmb,
    /// Disease-specific research (`DUO:0000007`, DS).
    Ds,
    /// Population origins or ancestry research only (`DUO:0000011`, POA).
    Poa,
    /// Research-specific restrictions modifier (`DUO:0000012`, RS).
    Rs,
    /// No general methods research (`DUO:0000015`, NMDS).
    Nmds,
    /// Genetic studies only (`DUO:0000016`, GSO).
    Gso,
    /// Not-for-profit, non-commercial use only (`DUO:0000018`, NPUNCU).
    Npuncu,
    /// Publication required (`DUO:0000019`, PUB).
    Pub,
    /// Collaboration required (`DUO:0000020`, COL).
    Col,
    /// Ethics approval required (`DUO:0000021`, IRB).
    Irb,
    /// Geographic restriction modifier (`DUO:0000022`, GS).
    Gs,
    /// Publication moratorium (`DUO:0000024`, MOR).
    Mor,
    /// Time limit on use (`DUO:0000025`, TS).
    Ts,
    /// User-specific restriction (`DUO:0000026`, US).
    Us,
    /// Project-specific restriction (`DUO:0000027`, PS).
    Ps,
    /// Institution-specific restriction (`DUO:0000028`, IS).
    Is,
    /// Return to database or resource (`DUO:0000029`, RTN).
    Rtn,
    /// General research use (`DUO:0000042`, GRU).
    Gru,
    /// Clinical care use (`DUO:0000043`, CC).
    Cc,
    /// Population origins or ancestry research prohibited (`DUO:0000044`, NPOA).
    Npoa,
    /// Not-for-profit organisation use only (`DUO:0000045`, NPU).
    Npu,
    /// Non-commercial use only (`DUO:0000046`, NCU).
    Ncu,
    /// Obsolete: general research use and clinical care (`DUO:0000005`, GRU-CC).
    GruCc,
    /// Obsolete: research use only (`DUO:0000014`, RU).
    Ru,
}

impl DuoCode {
    /// Returns the canonical DUO shorthand string (e.g. `"GRU"`, `"GRU-CC"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nres => "NRES",
            Self::Hmb => "HMB",
            Self::Ds => "DS",
            Self::Poa => "POA",
            Self::Rs => "RS",
            Self::Nmds => "NMDS",
            Self::Gso => "GSO",
            Self::Npuncu => "NPUNCU",
            Self::Pub => "PUB",
            Self::Col => "COL",
            Self::Irb => "IRB",
            Self::Gs => "GS",
            Self::Mor => "MOR",
            Self::Ts => "TS",
            Self::Us => "US",
            Self::Ps => "PS",
            Self::Is => "IS",
            Self::Rtn => "RTN",
            Self::Gru => "GRU",
            Self::Cc => "CC",
            Self::Npoa => "NPOA",
            Self::Npu => "NPU",
            Self::Ncu => "NCU",
            Self::GruCc => "GRU-CC",
            Self::Ru => "RU",
        }
    }

    /// Returns the OBO identifier for this term (e.g. `"DUO:0000042"`).
    pub fn obo_id(&self) -> &'static str {
        match self {
            Self::Nres => "DUO:0000004",
            Self::Hmb => "DUO:0000006",
            Self::Ds => "DUO:0000007",
            Self::Poa => "DUO:0000011",
            Self::Rs => "DUO:0000012",
            Self::Ru => "DUO:0000014",
            Self::Nmds => "DUO:0000015",
            Self::Gso => "DUO:0000016",
            Self::Npuncu => "DUO:0000018",
            Self::Pub => "DUO:0000019",
            Self::Col => "DUO:0000020",
            Self::Irb => "DUO:0000021",
            Self::Gs => "DUO:0000022",
            Self::Mor => "DUO:0000024",
            Self::Ts => "DUO:0000025",
            Self::Us => "DUO:0000026",
            Self::Ps => "DUO:0000027",
            Self::Is => "DUO:0000028",
            Self::Rtn => "DUO:0000029",
            Self::GruCc => "DUO:0000005",
            Self::Gru => "DUO:0000042",
            Self::Cc => "DUO:0000043",
            Self::Npoa => "DUO:0000044",
            Self::Npu => "DUO:0000045",
            Self::Ncu => "DUO:0000046",
        }
    }

    /// Returns `true` if this DUO term has been marked obsolete in the ontology.
    pub fn is_obsolete(&self) -> bool {
        matches!(self, Self::GruCc | Self::Ru)
    }

    /// Returns all known DUO codes.
    pub fn all() -> &'static [Self] {
        &[
            Self::Nres,
            Self::Hmb,
            Self::Ds,
            Self::Poa,
            Self::Rs,
            Self::Nmds,
            Self::Gso,
            Self::Npuncu,
            Self::Pub,
            Self::Col,
            Self::Irb,
            Self::Gs,
            Self::Mor,
            Self::Ts,
            Self::Us,
            Self::Ps,
            Self::Is,
            Self::Rtn,
            Self::Gru,
            Self::Cc,
            Self::Npoa,
            Self::Npu,
            Self::Ncu,
            Self::GruCc,
            Self::Ru,
        ]
    }
}

impl fmt::Display for DuoCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when parsing an unknown DUO code string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown DUO code: {0}")]
pub struct DuoCodeError(pub String);

impl FromStr for DuoCode {
    type Err = DuoCodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NRES" => Ok(Self::Nres),
            "HMB" => Ok(Self::Hmb),
            "DS" => Ok(Self::Ds),
            "POA" => Ok(Self::Poa),
            "RS" => Ok(Self::Rs),
            "NMDS" => Ok(Self::Nmds),
            "GSO" => Ok(Self::Gso),
            "NPUNCU" => Ok(Self::Npuncu),
            "PUB" => Ok(Self::Pub),
            "COL" => Ok(Self::Col),
            "IRB" => Ok(Self::Irb),
            "GS" => Ok(Self::Gs),
            "MOR" => Ok(Self::Mor),
            "TS" => Ok(Self::Ts),
            "US" => Ok(Self::Us),
            "PS" => Ok(Self::Ps),
            "IS" => Ok(Self::Is),
            "RTN" => Ok(Self::Rtn),
            "GRU" => Ok(Self::Gru),
            "CC" => Ok(Self::Cc),
            "NPOA" => Ok(Self::Npoa),
            "NPU" => Ok(Self::Npu),
            "NCU" => Ok(Self::Ncu),
            "GRU-CC" => Ok(Self::GruCc),
            "RU" => Ok(Self::Ru),
            other => Err(DuoCodeError(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duo_code_display_and_from_str() {
        for code in DuoCode::all() {
            let s = code.to_string();
            let parsed = DuoCode::from_str(&s).expect("from_str");
            assert_eq!(*code, parsed);
        }
    }

    #[test]
    fn duo_code_serde_round_trip() {
        for code in DuoCode::all() {
            let json = serde_json::to_string(code).expect("serialize");
            let decoded: DuoCode = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*code, decoded);
        }
    }

    #[test]
    fn duo_code_serde_uses_shorthand_strings() {
        assert_eq!(
            serde_json::to_string(&DuoCode::Gru).expect("serialize"),
            "\"GRU\""
        );
        assert_eq!(
            serde_json::to_string(&DuoCode::GruCc).expect("serialize"),
            "\"GRU-CC\""
        );
    }
}
