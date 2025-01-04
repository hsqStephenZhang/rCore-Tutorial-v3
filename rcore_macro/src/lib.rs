extern crate proc_macro;

use proc_macro::TokenStream;
use std::str::FromStr;

#[proc_macro_attribute]
pub fn init_section(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse level from attribute like "level = 3"
    let level = attr
        .to_string()
        .trim()
        .strip_prefix("level")
        .and_then(|s| s.trim().strip_prefix('='))
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1);

    // Convert the input tokens to a string
    let input = item.to_string();

    // Extract function name from the input
    let fn_name = input
        .split_whitespace()
        .skip_while(|&s| s != "fn")
        .nth(1)
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_');

    // Create uppercase version of function name
    let fn_name_upper = fn_name.to_uppercase();

    // Generate the complete output with both function and static variable
    let output = format!(
        "#[link_section = \".init.text\"]\n\
         #[no_mangle]\n\
         {}\n\
         #[link_section = \".initcall{}.init\"]\n\
         #[no_mangle]\n\
         #[used]\n\
         static {}_INITCALL: unsafe fn() -> i32 = {};\n",
        input, level, fn_name_upper, fn_name
    );

    // Parse back into a TokenStream
    TokenStream::from_str(&output).unwrap()
}

// it may disable the rust-analyzer's working on the function, so it's not recommended
#[proc_macro_attribute]
pub fn export_func(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Convert the input tokens to a string
    let input = item.to_string();
    // Extract function name from the input
    let fn_name = input
        .split_whitespace()
        .skip_while(|&s| s != "fn")
        .nth(1)
        .unwrap_or("")
        .split('(')  // Split at opening parenthesis to remove arguments
        .next()      // Take the part before the parenthesis
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
    
    // Fixed the raw string termination and formatting
    let asm = format!(
        "core::arch::global_asm!(
                r#\"
                .section \"__ksymtab_strings\",\"aMS\",%progbits,1
                __kstrtab_{fn_name}:
                    .asciz \"{fn_name}\"
                .previous
                .section \"___ksymtab\", \"a\"
                .balign 8
                __ksymtab_{fn_name}:
                    .long {fn_name}
                    .long __kstrtab_{fn_name}
                .previous
                \"#
            );"
    );

    // Combine the input function with the generated assembly
    let output = format!(
        "
        #[no_mangle]
        {input}
        {asm}
        "
    );

    // Parse back into a TokenStream
    TokenStream::from_str(&output).unwrap()
}
