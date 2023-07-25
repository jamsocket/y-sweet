use lib0::any::Any;
use std::collections::HashMap;
use yrs::{types::ToJson, Array, ReadTxn};

fn dump_object_inner(result: &Any, indent: usize) {
    let indent_str = "  ".repeat(indent);

    match result {
        Any::Map(map) => {
            println!("{{");
            for (key, value) in map.iter() {
                print!("  {}{}: ", indent_str, key);
                dump_object_inner(value, indent + 1);
            }
            println!("{}}}", indent_str);
        }
        Any::Array(array) => {
            println!("[");
            for value in array.iter() {
                print!("{}  ", indent_str);
                dump_object_inner(value, indent + 1);
            }
            println!("{}]", indent_str);
        }
        Any::String(string) => {
            println!("\"{}\"", string.replace('\"', "\\\""));
        }
        Any::Number(number) => {
            println!("{}", number);
        }
        Any::Bool(boolean) => {
            println!("{}", boolean);
        }
        Any::Null => {
            println!("null");
        }
        Any::Undefined => todo!(),
        Any::BigInt(_) => todo!(),
        Any::Buffer(_) => todo!(),
    }
}

fn dump_object(result: &Any) {
    dump_object_inner(result, 0);
}

pub fn dump<T: ReadTxn>(txn: &T) {
    let root_keys = txn.root_keys();

    let mut map: HashMap<String, Any> = HashMap::new();

    for key in root_keys {
        let value = txn.get_array(key).expect("Failed to get array");
        if value.len(txn) > 0 {
            map.insert(key.to_string(), value.to_json(txn));
            continue;
        }

        let value = txn.get_map(key).expect("Failed to get map");
        map.insert(key.to_string(), value.to_json(txn));
    }

    let result = Any::Map(Box::new(map));

    dump_object(&result);
}
