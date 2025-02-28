// #![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use hashbrown::HashMap;

pub fn encode(input: &String) -> String {
    let encoded: Vec<&str> = input.split(",")
        .collect::<Vec<&str>>();
    let n = encoded.len();
    let mut encoding = HashMap::<String, usize>::new();
    let mut uniques = Vec::new();
    let mut counter = 0;

    for i in 0..n {
        if encoding.contains_key(encoded[i]) { continue; }

        encoding.insert(encoded[i].to_string(), counter);
        uniques.push(encoded[i].to_string());
        counter += 1;
    }

    let mut result = String::with_capacity(uniques.capacity() * 2);
    let mut i = 0;
    let mut j = 0;

    for (_, key) in uniques.iter().enumerate() {
        result.push_str(key);
        if j != uniques.len() - 1 { result.push_str(","); }
        j += 1;
    }

    result.push_str(":");

    while i < n {
        result.push_str(encoding.get(encoded[i]).unwrap().to_string().as_str());
        i += 1;

        if i != n { result.push_str(","); }
    }

    result
}
pub fn decode(input: &String) -> String {
    let parts = input.split(":")
        .collect::<Vec<&str>>();
    let uniques = parts[0]
        .split(",")
        .collect::<Vec<&str>>();
    let codes = parts[1]
        .split(",")
        .map(|i| i.parse::<usize>().unwrap())
        .collect::<Vec<usize>>();

    // We trade some more memory for speed improvements
    let mut result = String::with_capacity(input.len() * 2);
    let n = codes.len();

    for i in 0..n {
        result.push_str(uniques[codes[i]]);
        if i != n-1 { result.push_str(","); }
    }

    result
}
