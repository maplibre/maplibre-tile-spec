
use dictionary::encode;

#[test]
fn test_encode_dictionary() {
    let input = String::from("USA,USA,USA,USA,Mexico,Canada,Mexico,Mexico,Mexico,Argentina");
    let expected_unique = vec!["USA", "Mexico", "Canada", "Argentina"];
    let expected_order = "0,0,0,0,1,2,1,1,1,3";

    let encoded = encode(&input);
    let encoded_split = encoded.split(":").collect::<Vec<&str>>();
    
    for unique in encoded_split[0].split(",") {
        if ! expected_unique.contains(&unique) { assert!(false); }
    }
    
    assert_eq!(encoded_split[1], expected_order);
}
