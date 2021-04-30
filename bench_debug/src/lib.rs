#![feature(proc_macro_def_site)]
use proc_macro::TokenStream;
use syn::{parse_quote};
use quote::quote;
use proc_macro2::{Span, Ident};

#[proc_macro_attribute]
pub fn log_bench(attrs: TokenStream, item: TokenStream) -> TokenStream {
	let capture_vars: Option<Ident> = syn::parse_macro_input!(attrs);

	let wrapped = func_wrap::parse_and_func_wrap_with(item, |func, wrapped_func| {
		let mut wrapped_func = if let Some(it) = wrapped_func {
			it
		} else {
			return Err(syn::Error::new(Span::call_site(), "Invalid attribute location"));
		};
		func.sig = wrapped_func.sig.clone();
		let name: String = func.sig.ident.to_string();
		let fname = &wrapped_func.sig.ident;
		let renamed = ::quote::format_ident!("__bench_internal_{}", fname);
		wrapped_func.sig.ident = renamed;

		if let Some(id) = &capture_vars {
			func.block = parse_quote!({
				// Store the debug string before calling the func since most inputs do not impl Copy
				let mut parsed_arg: String = format!("{:?}", &#id);
				// if !(parsed_arg.starts_with('"') && parsed_arg.ends_with('"')) {
				// 	parsed_arg = format!("\"{}\"", parsed_arg);
				// }

				let now = Instant::now();
				let ret = #wrapped_func;
				info!("Func {} with arg {} took {:.3} seconds", #name, parsed_arg, now.elapsed().as_secs_f64());
				ret
			});
		} else {
			func.block = parse_quote!({
				// info!("Entering func {}", #name);
				let now = Instant::now();
				let ret = #wrapped_func;
				info!("Func {} took {:.3} seconds", #name, now.elapsed().as_secs_f64());
				ret
			});
		}
		Ok(())
	}).unwrap();
	quote!(#wrapped).into()
}

