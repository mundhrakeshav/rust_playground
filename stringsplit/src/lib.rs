const STATIC_STR: &'static str = "";
fn print_type<T>(_: T) {
    println!("type: {}", std::any::type_name::<T>())
}
//  if 'a is lifetime of StrSplit
//  remainder and delimiter live for atleast 'a
pub struct StrSplit<'haystack, 'delimiter> {
    remainder: Option<&'haystack str>,
    delimiter: &'delimiter str,
}

impl<'haystack, 'delimiter> StrSplit<'haystack, 'delimiter> {
    pub fn new(haystack: &'haystack str, delimiter: &'delimiter str) -> Self {
        StrSplit {
            remainder: Some(haystack),
            delimiter,
        }
    }
}

impl<'haystack, 'delimiter> Iterator for StrSplit<'haystack, 'delimiter> {
    // If StrSplit is valid for 'a
    // then return `Item` lives for 'a
    type Item = &'haystack str;

    // fn next(&mut self) -> Option<Self::Item> {
    //     if let Some(next_delim) = self.remainder.find(self.delimiter) {
    //         let until_delimiter = &self.remainder[..next_delim];
    //         self.remainder = &self.remainder[next_delim + self.delimiter.len()..];
    //         Some(until_delimiter)
    //     } else if self.remainder.is_empty() {
    //         None
    //     } else {
    //         let rest = self.remainder;
    //         // self.remainder = "";
    //         // &'a str     = &'static str
    //         self.remainder = STATIC_STR;
    //         // self.remainder = ""; == self.remainder = STATIC_STR;
    //         // The above is allowed as 'static will live longer than 'a
    //         Some(rest)
    //     }
    // }

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut remainder) = self.remainder {
            if let Some(next_delim) = remainder.find(self.delimiter) {
                let until_delimiter = &remainder[..next_delim];
                *remainder = &remainder[next_delim + self.delimiter.len()..];
                Some(until_delimiter)
            } else {
                self.remainder.take()
                // None
            }
        } else {
            None
        }
    }
}

// fn until_character<'a>(s: &'a str, c: char) -> &'a str {
fn until_character<'a>(s: &str, c: char) -> &str {
    //  Err: cannot return value referencing temporary value returns a value referencing data owned by the current function
    //  StrSplit::new(s, c.to_string().as_str()).next().unwrap()
    //  The above logic makes rust think `s` and `c` have same lifetime
    let delimiter = format!("{}", c);
     StrSplit::new(s, &delimiter).next().unwrap()

}

#[test]
fn test_it_works() {
    let haystack = "a b c d e";

    let letters: Vec<_> = StrSplit::new(haystack, " ").collect();
    assert_eq!(letters, vec!["a", "b", "c", "d", "e"])
}

#[test]
fn test_char_test() {
    assert_eq!(until_character("hello world", 'o'), "hell")
}
