
mod large {
    use std::fs;

    use fastpfor_rs::FastPFOR;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref ASSETS_LARGE_RAW: Vec<u32> = {
            let data = fs::read_to_string("./assets/large_raw.txt")
                .expect("Error: can not read from assets (large_raw.txt)");
            data.split(",")
                .map(|i| { if i.parse::<i32>().is_err() {eprintln!("{}", i);} i.parse::<i32>().unwrap() as u32 })
                .collect()
        };
        static ref ASSETS_LARGE_ENCODED: Vec<u32> = {
            let data = fs::read_to_string("./assets/large_fastpfor.txt")
                .expect("Error: can not read from assets (large_fastpfor.txt)");
            data.split(",")
                .map(|i| { if i.parse::<i32>().is_err() {eprintln!("{}", i);} i.parse::<i32>().unwrap() as u32 })
                .collect()
        };
    }

    #[test]
    fn decode() {
        let mut core = FastPFOR::default();
        let mut output = vec![0; ASSETS_LARGE_RAW.len()];

        core.decode(&ASSETS_LARGE_ENCODED, &mut output);

        assert_eq!(output, ASSETS_LARGE_RAW.as_slice());
    }
}
