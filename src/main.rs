use udashboard::v1;

fn main() {
    let config = v1::load("config.ron".to_string()).unwrap();
    println!("Raw: {:?}", config);

    // connect to data source

    // set up screen

    // start update loop.
}
