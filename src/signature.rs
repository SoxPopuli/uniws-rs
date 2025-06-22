use crate::error::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MatchType {
    Exact,
    Wild,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Signature {
    pub pattern: Vec<Option<u8>>,
}
impl Signature {
    pub fn from_string(section: &str, signature: &str, sigwild: &str) -> Result<Self, Error> {
        fn read_sig(section: &str, sig: &str) -> Result<Vec<u8>, Error> {
            (0..sig.len())
                .step_by(2)
                .map(|x| {
                    if x + 1 >= sig.len() {
                        return Err(Error::config_field_parse(
                            section,
                            "sig",
                            "Invalid hex string length",
                        ));
                    }

                    let byte_pair = &sig[x..=x + 1];

                    u8::from_str_radix(byte_pair, 16).map_err(|_| {
                        Error::config_field_parse(
                            section,
                            "sig",
                            format!("Invalid hex byte pair: {byte_pair}"),
                        )
                    })
                })
                .collect()
        }

        fn read_sigwild(section: &str, sigwild: &str) -> Result<Vec<MatchType>, Error> {
            sigwild
                .chars()
                .map(|c| match c {
                    '0' => Ok(MatchType::Exact),
                    '1' => Ok(MatchType::Wild),
                    x => Err(Error::config_field_parse(
                        section,
                        "sigwild",
                        format!("Invalid sigwild character: {x}"),
                    )),
                })
                .collect::<Result<Vec<_>, _>>()
        }

        let sig = read_sig(section, signature)?;
        let sigwild = read_sigwild(section, sigwild)?;
        Ok(Self::new(&sig, &sigwild))
    }

    pub fn new(signature: &[u8], sigwild: &[MatchType]) -> Self {
        assert_eq!(signature.len(), sigwild.len());

        let pattern = signature
            .iter()
            .zip(sigwild)
            .map(|(sig, wild)| match wild {
                MatchType::Wild => None,
                MatchType::Exact => Some(*sig),
            })
            .collect();

        Self { pattern }
    }

    fn search_at(&self, haystack: &[u8], index: usize) -> Option<usize> {
        if self.pattern.len() > haystack[index..].len() {
            return None;
        }

        for i in 0..self.pattern.len() {
            match self.pattern[i] {
                None => continue, //Wildcard
                Some(byte) => {
                    if byte == haystack[i + index] {
                        continue;
                    } else {
                        return None;
                    }
                }
            }
        }

        Some(index)
    }

    pub fn try_find(&self, haystack: &[u8]) -> Option<usize> {
        for i in 0..haystack.len() {
            if haystack.len() - i < self.pattern.len() {
                return None;
            }

            match self.search_at(haystack, i) {
                Some(index) => return Some(index),
                None => continue,
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_signature() -> Signature {
        let sig = [0x80, 0x02, 0x00, 0x00, 0xC7, 0x01, 0xE0, 0x01, 0x00, 0x00];
        let sigwild = [
            MatchType::Exact,
            MatchType::Exact,
            MatchType::Exact,
            MatchType::Exact,
            MatchType::Wild,
            MatchType::Wild,
            MatchType::Exact,
            MatchType::Exact,
            MatchType::Exact,
            MatchType::Exact,
        ];
        Signature::new(&sig, &sigwild)
    }

    #[test]
    fn create() {
        let sig = get_signature();

        #[rustfmt::skip]
        assert_eq!(sig.pattern, [
            Some(0x80), Some(0x02), Some(0x00), Some(0x00), 
            None, None,
            Some(0xE0), Some(0x01), Some(0x00), Some(0x00), 
        ]);

        let sig_from_string =
            Signature::from_string("test", "80020000C701E0010000", "0000110000").unwrap();

        assert_eq!(sig, sig_from_string);
    }

    #[test]
    fn match_test() {
        let sig = get_signature();

        #[rustfmt::skip]
        let has_sig = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x80, 0x02, 0x00, 0x00, 0x00, 0x00,

            0x80, 0x02, 0x00, 0x00, 0xC7, 0x01, 0xE0, 0x01, 0x00, 0x00,

            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        assert_eq!(sig.try_find(&has_sig), Some(20));

        #[rustfmt::skip]
        let doesnt_have_sig = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        assert_eq!(sig.try_find(&doesnt_have_sig), None);

        #[rustfmt::skip]
        let has_sig_at_end = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x80, 0x02, 0x00, 0x00, 0xC7, 0x01, 0xE0, 0x01, 0x00, 0x00,
        ];

        assert_eq!(sig.try_find(&has_sig_at_end), Some(10));

        #[rustfmt::skip]
        let sig_only = [
            0x80, 0x02, 0x00, 0x00, 0xC7, 0x01, 0xE0, 0x01, 0x00, 0x00,
        ];
        assert_eq!(sig.try_find(&sig_only), Some(0));
    }
}
