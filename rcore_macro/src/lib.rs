extern crate proc_macro;

use proc_macro::TokenStream;
use std::str::FromStr;

#[proc_macro_attribute]
pub fn init_section(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse level from attribute like "level = 3"
    let level = attr.to_string()
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
