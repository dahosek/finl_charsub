use criterion::{criterion_group, criterion_main, Criterion};
use std::io::Cursor;
use finl_charsub::charsub::CharSubMachine;
use std::path::PathBuf;
use std::fs;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("resources");
    path.push("tex.charsub");
    let charsub_data = fs::read_to_string(path).unwrap();
//    let mut char_sub_machine = CharSubMachine::from_buf_reader(&mut BufReader::new(&charsub_data)).unwrap();

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("resources");
    path.push("sample.tex");
    let input_data = fs::read_to_string(path).unwrap();

    c.bench_function("Process sample TeX file",
        |b| b.iter(|| {
            let mut char_sub_machine = CharSubMachine::from_buf_reader(&mut Cursor::new(&charsub_data)).unwrap();
            char_sub_machine.process(&input_data.as_str())
        })
    );

    // c.bench_with_input(BenchmarkId::new("Process article","(input_data and charsub_data)"),
    //                    (input_data, charsub_data)
    //                     |b, input|
    //                         { b.iter(|| {
    //                             let (charsub_data, input_data) = input;
    //                             let mut char_sub_machine = CharSubMachine::from_buf_reader(&mut Cursor::new(&charsub_data)).unwrap();
    //                             char_sub_machine.process(&input_data.as_str());
    //                         })
    //                     });


}



criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

