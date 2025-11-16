use autonomi::Bytes;
use postem::{ConnectionType, PostemClient, addressee::PostemName};

#[tokio::test]
async fn test_sending_and_receiving() {
    // Assumes run from freshly started local client
    let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
    let payment_option = client.get_payment_option("").unwrap();
    let name = "integration.test.address";
    let addressee = client
        .addressee_create(name, payment_option.clone(), None)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let mut door_mat = client
        .doormat_init(addressee.secret_key(), name)
        .await
        .unwrap();
    // test with nothing posted
    assert!(door_mat.items().is_empty());
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    // test after adding a package
    eprintln!(
        "last received pk with no pacakges : {}",
        addressee.last_received().owner().to_hex()
    );
    let message = Bytes::from("Hello");
    let route = client
        .route_get(PostemName::create(name).unwrap(), true)
        .await
        .unwrap();
    let next_location_pk = client
        .location_get_available(route.clone())
        .await
        .unwrap()
        .public_key()
        .to_hex();
    let (package, attos) = client
        .package_post(addressee.address(), message.clone(), payment_option.clone())
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    eprintln!("cost of posting: {attos}");
    eprintln!(
        "pk of pacakge location            : {}",
        package.address().owner.to_hex()
    );
    eprintln!(
        "Location pk from next location pre post :{}",
        next_location_pk
    );
    let next_location_pk = client
        .location_get_available(route)
        .await
        .unwrap()
        .public_key()
        .to_hex();
    eprintln!(
        "Location pk from next location post post:{}",
        next_location_pk
    );
    eprintln!(
        "last received pk                  : {}",
        addressee.last_received().owner().to_hex()
    );
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), 1);
    assert_eq!(
        door_mat.items().first().unwrap().payload().unwrap(),
        message
    );

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
