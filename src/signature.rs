use crate::patch_info::MatchType;

#[derive(Debug, PartialEq, Eq)]
pub struct Signature {
    pub pattern: Vec<Option<u8>>,
}
impl Signature {
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
