use failure::Error;
use gluon::{
    base::{
        ast::{ Expr, Lambda, TypedIdent },
        pos,
        types::TypeCache,
    },
    compiler_pipeline::Executable,
    vm::{
        api::{ Function, Getable, OwnedFunction, VmType },
        Variants,
    },
    Compiler,
    Future,
    RootedThread,
};

pub fn add_prelude<A: VmType, R: VmType>(args: &[&str], name: &str, script: &str, compiler: &mut Compiler, thread: RootedThread) -> Result<Function<RootedThread, fn(A) -> R>, Error> {
    let mut expr = compiler.parse_expr(&TypeCache::new(), name, script)?;

    {
        let span = pos::Span::with_id(0.into(), 0.into(), pos::UNKNOWN_EXPANSION);
        let symbols = compiler.mut_symbols();
        expr = pos::spanned(
            span,
            Expr::Lambda(Lambda {
                id: TypedIdent::new(symbols.symbol(name)),
                args: args.iter().map(|a| pos::spanned(span, TypedIdent::new(symbols.symbol(*a)))).collect(),
                body: Box::new(expr),
            }),
        );
    }

    let expected_type = OwnedFunction::<fn(A) -> R>::make_type(&thread);
    let execute_value = expr.run_expr(
        compiler,
        thread.clone(),
        name,
        script,
        Some(&expected_type),
    ).wait()?;

    Ok(OwnedFunction::from_value(&thread, unsafe {
        Variants::new(&*execute_value.value)
    }))
}
