use postem::{ConnectionType, PostemClient};

#[tokio::test]
async fn test_sending_and_receiving() {
    // Assumes run from freshly started local client
    let client = PostemClient::init(ConnectionType::Local).await.unwrap();
    let payment_option = client.get_payment_option("").await.unwrap();
    let addressee = client
        .addressee_create("integration.test.address", payment_option.clone(), None)
        .await
        .unwrap();
    let route = client.route_get(addressee.address(), false).await.unwrap();
    let route_contents = client.route_get_packages(route).await.unwrap();
    assert!(route_contents.is_empty());
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    // test current_location after adding a package
    // let next_location = client.location_get_available(route).await.unwrap();
    // let (package, _) = client
    //     .package_post(addressee.address(), Bytes::from("Hello"), payment_option)
    //     .await
    //     .unwrap();
    // tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    // let inspected_pacakges = client.
    // after adding a pointer (ie not valid pacakge)
    // after adding another package.
}

// test sending to non-existant address
