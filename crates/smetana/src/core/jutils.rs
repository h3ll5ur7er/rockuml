//! `JUtils`: Smetana's replacements for the C library.

/// `JUtils.qsort` / `qsortInt`: not libc's quicksort but a stable bubble sort, which decides the order of equal
/// elements. Like Java, it fails if the comparator is inconsistent (the array is not sorted afterwards).
pub fn qsort<T: Copy>(array: &mut [T], mut compare: impl FnMut(T, T) -> i32) {
    let nb = array.len();
    for _pass in 0..nb.saturating_sub(1) {
        let mut change = false;
        for i in 0..nb - 1 {
            if compare(array[i], array[i + 1]) > 0 {
                change = true;
                array.swap(i, i + 1);
            }
        }
        if !change {
            break;
        }
    }
    for i in 0..nb.saturating_sub(1) {
        assert!(
            compare(array[i], array[i + 1]) <= 0,
            "qsort: inconsistent comparator"
        );
    }
}

/// `CString.strcmp`: compares UTF-16 code units up to the terminating NUL and returns their difference.
pub fn strcmp(s1: &str, s2: &str) -> i32 {
    let mut a = s1.encode_utf16().chain(std::iter::once(0));
    let mut b = s2.encode_utf16().chain(std::iter::once(0));
    loop {
        let (ca, cb) = (a.next().unwrap_or(0), b.next().unwrap_or(0));
        let diff = i32::from(ca) - i32::from(cb);
        if ca == 0 || diff != 0 {
            return diff;
        }
    }
}

/// `atof`, which Smetana implements with `Double.parseDouble`: surrounding whitespace and control characters are
/// ignored and a `d`/`f` type suffix is accepted. Java throws on anything else; so does this.
pub fn atof(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c <= ' ');
    let digits = t.strip_suffix(['d', 'D', 'f', 'F']).unwrap_or(t);
    let finite = digits.trim_start_matches(['+', '-']);
    let parsed = if finite.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        digits.parse::<f64>().ok()
    } else {
        match finite {
            "Infinity" if t == digits => digits.replace("Infinity", "inf").parse::<f64>().ok(),
            "NaN" if t == digits => Some(f64::NAN),
            _ => None,
        }
    };
    parsed.unwrap_or_else(|| panic!("NumberFormatException: {s:?}"))
}

/// `atoi`, which Smetana implements with `Integer.parseInt`: an optional sign and decimal digits, nothing else.
pub fn atoi(s: &str) -> i32 {
    s.parse::<i32>()
        .unwrap_or_else(|_| panic!("NumberFormatException: {s:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qsort_is_a_stable_bubble_sort() {
        let mut v = [(3, 'a'), (1, 'b'), (3, 'c'), (1, 'd'), (2, 'e')];
        qsort(&mut v, |x, y| x.0 - y.0);
        assert_eq!(v, [(1, 'b'), (1, 'd'), (2, 'e'), (3, 'a'), (3, 'c')]);
    }

    #[test]
    #[should_panic(expected = "inconsistent comparator")]
    fn qsort_rejects_inconsistent_comparators() {
        let mut v = [1, 2, 3];
        qsort(&mut v, |_, _| 1);
    }

    #[test]
    fn strcmp_compares_utf16_code_units() {
        assert!(strcmp("abc", "abd") < 0);
        assert_eq!(strcmp("abc", "abc"), 0);
        assert_eq!(strcmp("ab", "abc"), -i32::from(b'c'));
        // U+FFFD sorts after a surrogate pair in UTF-16, unlike in UTF-8.
        assert!(strcmp("\u{FFFD}", "\u{1F600}") > 0);
    }

    #[test]
    fn atof_parses_like_java() {
        assert_eq!(atof(".75"), 0.75);
        assert_eq!(atof(" 16 "), 16.0);
        assert_eq!(atof("0.630301534628363"), 0.630301534628363);
        assert_eq!(atof("2d"), 2.0);
        assert_eq!(atof("-1e3"), -1000.0);
        assert!(atof("-Infinity").is_infinite());
        assert!(atof("NaN").is_nan());
    }

    #[test]
    #[should_panic(expected = "NumberFormatException")]
    fn atof_rejects_rust_only_spellings() {
        atof("inf");
    }

    #[test]
    fn atoi_parses_like_java() {
        assert_eq!(atoi("-12"), -12);
        assert_eq!(atoi("+7"), 7);
    }
}
