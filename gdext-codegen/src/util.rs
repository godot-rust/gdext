use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

pub fn to_module_name(class_name: &str) -> String {
    // Remove underscores and make peekable
    let mut class_chars = class_name.bytes().filter(|&ch| ch != b'_').peekable();

    // 2-lookbehind
    let mut previous: [Option<u8>; 2] = [None, None]; // previous-previous, previous

    // None is not upper or number
    #[inline(always)]
    fn is_upper_or_num<T>(ch: T) -> bool
    where
        T: Into<Option<u8>>,
    {
        let ch = ch.into();
        match ch {
            Some(ch) => ch.is_ascii_digit() || ch.is_ascii_uppercase(),
            None => false,
        }
    }

    // None is lowercase
    #[inline(always)]
    fn is_lower_or<'a, T>(ch: T, default: bool) -> bool
    where
        T: Into<Option<&'a u8>>,
    {
        let ch = ch.into();
        match ch {
            Some(ch) => ch.is_ascii_lowercase(),
            None => default,
        }
    }

    let mut result = Vec::with_capacity(class_name.len());
    while let Some(current) = class_chars.next() {
        let next = class_chars.peek();

        let [two_prev, one_prev] = previous;

        // See tests for cases covered
        let caps_to_lowercase = is_upper_or_num(one_prev)
            && is_upper_or_num(current)
            && is_lower_or(next, false)
            && !is_lower_or(&two_prev, true);

        // Add an underscore for Lowercase followed by Uppercase|Num
        // Node2D => node_2d (numbers are considered uppercase)
        let lower_to_uppercase = is_lower_or(&one_prev, false) && is_upper_or_num(current);

        if caps_to_lowercase || lower_to_uppercase {
            result.push(b'_');
        }
        result.push(current.to_ascii_lowercase());

        // Update the look-behind
        previous = [previous[1], Some(current)];
    }

    let mut result = String::from_utf8(result).unwrap();

    // There are a few cases where the conversions do not work:
    // * VisualShaderNodeVec3Uniform => visual_shader_node_vec_3_uniform
    // * VisualShaderNodeVec3Constant => visual_shader_node_vec_3_constant
    if let Some(range) = result.find("_vec_3").map(|i| i..i + 6) {
        result.replace_range(range, "_vec3_")
    }
    if let Some(range) = result.find("gd_native").map(|i| i..i + 9) {
        result.replace_range(range, "gdnative")
    }
    if let Some(range) = result.find("gd_script").map(|i| i..i + 9) {
        result.replace_range(range, "gdscript")
    }

    // To prevent clobbering `gdnative` during a glob import we rename it to `gdnative_`
    if result == "gdnative" {
        return "gdnative_".into();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name_generator() {
        let tests = vec![
            // A number of test cases to cover some possibilities:
            // * Underscores are removed
            // * First character is always lowercased
            // * lowercase to an uppercase inserts an underscore
            //   - FooBar => foo_bar
            // * two capital letter words does not separate the capital letters:
            //   - FooBBaz => foo_bbaz (lower, cap, cap, lower)
            // * many-capital letters to lowercase inserts an underscore before the last uppercase letter:
            //   - FOOBar => boo_bar
            // underscores
            ("Ab_Cdefg", "ab_cdefg"),
            ("_Abcd", "abcd"),
            ("Abcd_", "abcd"),
            // first and last
            ("Abcdefg", "abcdefg"),
            ("abcdefG", "abcdef_g"),
            // more than 2 caps
            ("ABCDefg", "abc_defg"),
            ("AbcDEFg", "abc_de_fg"),
            ("AbcdEF10", "abcd_ef10"),
            ("AbcDEFG", "abc_defg"),
            ("ABCDEFG", "abcdefg"),
            ("ABC", "abc"),
            // Lowercase to an uppercase
            ("AbcDefg", "abc_defg"),
            // Only 2 caps
            ("ABcdefg", "abcdefg"),
            ("ABcde2G", "abcde_2g"),
            ("AbcDEfg", "abc_defg"),
            ("ABcDe2G", "abc_de_2g"),
            ("abcdeFG", "abcde_fg"),
            ("AB", "ab"),
            // Lowercase to an uppercase
            ("AbcdefG", "abcdef_g"), // PosX => pos_x
            // text changes
            ("FooVec3Uni", "foo_vec3_uni"),
            ("GDNative", "gdnative_"),
            ("GDScript", "gdscript"),
        ];
        tests.iter().for_each(|(class_name, expected)| {
            let actual = module_name_from_class_name(class_name);
            assert_eq!(*expected, actual, "Input: {}", class_name);
        });
    }
}

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

pub fn ident_escaped(s: &str) -> Ident {
    // note: could also use Ident::parse(s) from syn, but currently this crate doesn't depend on it

    let transformed = match s {
        "type" => "type_",
        s => s,
    };

    ident(transformed)
}

pub fn c_str(s: &str) -> TokenStream {
    let s = Literal::string(&format!("{}\0", s));
    quote! {
        #s.as_ptr() as *const i8
    }
}

pub fn strlit(s: &str) -> Literal {
    Literal::string(s)
}
