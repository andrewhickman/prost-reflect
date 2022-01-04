/// Convert `src` from snake case to camel case, returning an error if roundtripping
/// back to snake case would not be possible.
pub(crate) fn snake_case_to_camel_case(dst: &mut String, src: &str) -> Result<(), ()> {
    let mut ucase_next = false;
    for ch in src.chars() {
        if ch.is_ascii_uppercase() {
            return Err(());
        }

        if ucase_next {
            let upper_ch = ch.to_ascii_uppercase();
            if upper_ch == ch {
                return Err(());
            }

            dst.push(upper_ch);
            ucase_next = false;
        } else if ch == '_' {
            ucase_next = true;
        } else {
            dst.push(ch)
        }
    }

    Ok(())
}

pub(crate) fn camel_case_to_snake_case(result: &mut String, part: &str) -> Result<(), ()> {
    for ch in part.chars() {
        if ch.is_ascii_uppercase() {
            result.push('_');
            result.push(ch.to_ascii_lowercase());
        } else if ch == '_' {
            return Err(());
        } else {
            result.push(ch);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn snake_to_camel() {
        let mut buf = String::new();

        snake_case_to_camel_case(&mut buf, "foo").unwrap();
        assert_eq!(&buf, "foo");
        buf.clear();

        snake_case_to_camel_case(&mut buf, "foo_bar").unwrap();
        assert_eq!(&buf, "fooBar");
        buf.clear();
    }

    #[test]
    fn camel_to_snake() {
        let mut buf = String::new();

        camel_case_to_snake_case(&mut buf, "foo").unwrap();
        assert_eq!(&buf, "foo");
        buf.clear();

        camel_case_to_snake_case(&mut buf, "fooBar").unwrap();
        assert_eq!(&buf, "foo_bar");
        buf.clear();
    }

    #[test]
    fn bad_roundtrips() {
        let mut buf = String::new();
        assert!(snake_case_to_camel_case(&mut buf, "fooBar").is_err());
        assert!(snake_case_to_camel_case(&mut buf, "foo_3_bar").is_err());
        assert!(snake_case_to_camel_case(&mut buf, "foo__bar").is_err());
    }

    proptest! {
        #[test]
        fn roundtrip_cases(snake_case in "[a-zA-Z0-9]+") {
            let mut camel_case = String::new();
            if snake_case_to_camel_case(&mut camel_case, &snake_case).is_ok() {
                let mut roundtripped_snake_case = String::new();
                camel_case_to_snake_case(&mut roundtripped_snake_case, &camel_case).unwrap();

                prop_assert_eq!(snake_case, roundtripped_snake_case);
            }
        }
    }
}
