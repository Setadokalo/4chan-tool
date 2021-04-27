#![feature(proc_macro_def_site)]
use proc_macro::TokenStream;
use syn::{ItemFn, parse_macro_input};
use quote::{quote, format_ident};

#[proc_macro_attribute]
pub fn log_bench(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// let mut input_fn = parse_macro_input!(item as ItemFn);
	// // let mut statements: 
	// // modify_returns(&mut statements, &benchvar_name);
	// // panic!("{:#?}", statements.pop());
	// let fn_sig = input_fn.sig.clone();
	// let fn_name = format!("{}", fn_sig.ident);
	// let internal_fn_ident = format_ident!("_internal_{}_bench", input_fn.sig.ident);
	// input_fn.sig.ident = internal_fn_ident.clone();
	let wrapped = func_wrap::parse_and_func_wrap_with(item, |i, o| {
		Ok(())
	}).unwrap();
	quote!(#wrapped).into()
}

	// quote!(
	// 	#input_fn

	// 	#fn_sig {
	// 		let bench_start: std::time::Instant = std::time::Instant::now();
	// 		let return_value = #internal_fn_ident();
	// 		info!("fn {} returned in {:.4} seconds ", #fn_name, bench_start.elapsed().as_secs_f64());
	// 		return_value
	// 	}	
	// ).into()