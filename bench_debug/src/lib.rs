#![feature(proc_macro_def_site)]
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use syn::{Expr, ItemFn, Stmt, parse_macro_input};
use syn::parse_quote;
use quote::{quote, format_ident};

#[proc_macro_attribute]
pub fn log_bench(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let mut input_fn = parse_macro_input!(item as ItemFn);
	let mut statements: Vec<Stmt> = input_fn.block.stmts;
	
	let benchvar_name = format_ident!("__{}__BENCH", input_fn.sig.ident);

	modify_returns(&mut statements, &benchvar_name);

	let benchtime_create = parse_quote!({
		let #benchvar_name: Instant = std::time::Instant::now();
		#(#statements)*
	});
	input_fn.block.stmts = benchtime_create;

	// input_fn.block.stmts.insert(input_fn.block.stmts.len() - 2, benchtime_measure);

	quote!(#input_fn).into()
}

fn modify_returns(statements: &mut Vec<Stmt>, benchvar_name: &Ident) {
	let mut found_i = Vec::new();
	for (i, statement) in statements.iter_mut().enumerate() {
		match statement {
			Stmt::Expr(expr) => {
				panic!("{:?}", expr);
			}
			Stmt::Semi(expr, _) => {
				match_expr(expr, benchvar_name, i, &mut found_i);
			}
			_ => {}
		}
	}
	for i in found_i {
		panic!("found an i");
		let benchtime_measure = parse_quote!({
			info!();
			hsgrfjkgkljgfdhglkj
			// #benchvar_name = std::time::Instant::now();
		});
		statements.insert(i, benchtime_measure)
	}
}

fn match_expr(expr: &mut Expr, benchvar_name: &Ident, i: usize, found_i: &mut Vec<usize>) {
	match expr {
		syn::Expr::Return(_) => {
			found_i.push(i);
		}
		syn::Expr::Block(exprblock) => {
			modify_returns(&mut exprblock.block.stmts, benchvar_name);
		}
		syn::Expr::Field(_) => {}
		syn::Expr::ForLoop(exprfor) => {
			modify_returns(&mut exprfor.body.stmts, benchvar_name);
		}
		syn::Expr::Group(_) => {}
		syn::Expr::If(_) => {}
		syn::Expr::Loop(_) => {}
		syn::Expr::Match(_) => {}
		syn::Expr::Try(_) => {}
		syn::Expr::TryBlock(_) => {}
		syn::Expr::Unsafe(ublock) => {
			modify_returns(&mut ublock.block.stmts, benchvar_name);
		}
		// syn::Expr::While(_) => {}
		_ => {}
	}
}