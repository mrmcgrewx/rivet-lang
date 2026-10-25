use rivet::{
    language::{Execution, compile_binary as compile},
    runtime::ProgramServer,
};
use std::{hint::black_box, time::Instant};
fn main() {
    let source = include_bytes!("../examples/computed.rvb");
    let start = Instant::now();
    for _ in 0..2_000 {
        black_box(compile(black_box(source)).unwrap());
    }
    println!(
        "compile_computed: {} ns/op (2000 iterations)",
        start.elapsed().as_nanos() / 2_000
    );
    let program = compile(source).unwrap();
    let start = Instant::now();
    for _ in 0..20_000 {
        black_box(program.execute().unwrap());
    }
    println!(
        "execute_computed: {} ns/op (20000 iterations)",
        start.elapsed().as_nanos() / 20_000
    );
    let mut source = include_bytes!("../examples/server.rvb").to_vec();
    let offset = rivet::language::inspect_binary(&source)
        .unwrap()
        .entry_offset;
    source[offset..offset + 2].copy_from_slice(&0u16.to_le_bytes());
    let Execution::Http { port, routes } = compile(&source).unwrap().execute().unwrap() else {
        panic!()
    };
    let mut server = ProgramServer::start(port, routes).unwrap();
    let start = Instant::now();
    for _ in 0..500 {
        black_box(server.request("/hello").unwrap());
    }
    println!(
        "loopback_http: {} us/request (500 sequential new TCP connections)",
        start.elapsed().as_micros() / 500
    );
    server.stop().unwrap();
}
