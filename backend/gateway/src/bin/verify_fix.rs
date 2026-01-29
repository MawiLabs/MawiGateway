fn main() {
    let url = "https://example.com/image.png";
    let content_image = "Here is an image: ![Alt](https://example.com/image.png)";
    let content_link = "Here is an image: [Alt](https://example.com/image.png)";
    let content_raw = "Here is an image: https://example.com/image.png";

    // Logic replication from agentic_executor.rs
    let check = |content: &str| -> bool {
        let url_present = content.contains(url);
        if url_present {
              content.match_indices(url).any(|(idx, _)| {
                  if idx >= 2 {
                      let prefix = &content[idx-2..idx];
                      if prefix == "](" {
                          if let Some(last_open) = content[..idx-2].rfind('[') {
                              if last_open > 0 {
                                  if &content[last_open-1..last_open] == "!" {
                                      println!("  Trace: Found '!' at {}", last_open-1);
                                      return true; 
                                  } else {
                                      println!("  Trace: Char at {} is '{:?}'", last_open-1, &content[last_open-1..last_open]);
                                  }
                              } else {
                                  println!("  Trace: last_open is 0");
                              }
                              return false; 
                          }
                      }
                  }
                  false
              })
        } else {
            false
        }
    };

    println!("Checking IMAGE format:");
    let res_image = check(content_image);
    println!("Result: {}\n", res_image);
    assert!(res_image, "Should capture image format");

    println!("Checking LINK format:");
    let res_link = check(content_link);
    println!("Result: {}\n", res_link);
    assert!(!res_link, "Should REJECT link format");

    println!("Checking RAW format:");
    let res_raw = check(content_raw);
    println!("Result: {}\n", res_raw);
    assert!(!res_raw, "Should REJECT raw URL");

    println!("ALL LOGIC CHECKS PASSED ✅");
}
