pub trait Parse {
    fn int_parse(value: String) -> Self;
}

impl Parse for bool {
    #[inline]
    fn int_parse(value: String) -> Self {
        let v: usize = value.parse().unwrap_or(0);
        v == 1
    }
}
