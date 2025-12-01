use autonomi::pointer::PointerTarget;
use autonomi::{Bytes, Client, GraphEntryAddress, Pointer, SecretKey};
use postem::addressee::PostemName;
use postem::{ConnectionType, PostemClient, PostemError};
use std::time::SystemTime;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_sending_and_receiving() {
    let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
    let payment_option = client.get_payment_option("").unwrap();
    let name = SecretKey::random().to_hex();
    let (addressee, _) = client
        .addressee_create(&name, payment_option.clone(), None)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let mut door_mat = client
        .doormat_init(addressee.secret_key(), &name)
        .await
        .unwrap();
    // test with nothing posted
    assert!(door_mat.items().is_empty());
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    // test after adding a package
    let message = Bytes::from("Hello");
    let (_package, _attos) = client
        .package_post(addressee.address(), message.clone(), payment_option.clone())
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    // eprintln!("{:?}", door_mat.items().first().unwrap());
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), 1);
    assert_eq!(
        door_mat
            .items()
            .first()
            .expect("Should be something on doormat")
            .payload()
            .expect("message should have a payload"),
        message
    );
    // test update without any new packages should return same number
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    assert_eq!(door_mat.items().len(), 1);
    // eprintln!("{:?}", door_mat.items().first().unwrap());

    // after adding a pointer (ie not valid pacakage)
    let route = client
        .route_get(PostemName::create(&name).unwrap(), false)
        .await
        .unwrap();
    let next_location = client.location_get_available(route.clone()).await.unwrap();
    let pointer = Pointer::new(
        &next_location,
        0,
        PointerTarget::GraphEntryAddress(GraphEntryAddress::new(SecretKey::random().public_key())),
    );
    let autonomi_client = Client::init_local().await.unwrap();
    autonomi_client
        .pointer_put(pointer, payment_option.clone())
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    assert_eq!(door_mat.items().len(), 1);
    // after adding a scratchpad (ie not valid pacakage)
    let route = client
        .route_get(PostemName::create(&name).unwrap(), false)
        .await
        .unwrap();
    let next_location = client.location_get_available(route.clone()).await.unwrap();
    let data = Bytes::from("Hello");
    autonomi_client
        .scratchpad_create(&next_location, 0, &data, payment_option.clone())
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    assert_eq!(door_mat.items().len(), 1);
    // if I get around to working out how to simulate a chunk collision add a test for that too

    // after adding another package.
    // do a fresh pull of entire door mat
    let message2 = Bytes::from("Hello again");
    let (_package, _attos) = client
        .package_post(
            addressee.address(),
            message2.clone(),
            payment_option.clone(),
        )
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    // eprintln!("{:?}", door_mat.items().first().unwrap());
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), 2);
    assert_eq!(
        door_mat
            .items()
            .first()
            .expect("Should be something on doormat")
            .payload()
            .expect("message should have a payload"),
        message
    );
    assert_eq!(
        door_mat
            .items()
            .last()
            .expect("Should be something on doormat")
            .payload()
            .expect("message should have a payload"),
        message2
    );
}
// test sending to non-existant address
#[tokio::test]
async fn test_post_to_non_existing_address() {
    let client = PostemClient::init(ConnectionType::Local).await.unwrap();
    let payment_option = client.get_payment_option("").unwrap();
    let address = PostemName::create("I.don't.exist").unwrap();
    let message = Bytes::from("Hello");
    let post_result = client
        .package_post(address, message.clone(), payment_option.clone())
        .await;
    assert_eq!(
        post_result,
        Err(PostemError::InvalidAddress("I.don't.exist".to_string()))
    );
    // if issue with antnet would we get the same error...though how could we know otherwise if the network isn't returning data
    // Returns Err(GraphEntryError("Record could not be found."))}...which would Be GraphError::GetError(GetError::RecordNotFound)
}

// stress test adding thousands of pack:ges and thousands of non-pacakges
#[tokio::test]
#[ignore]
async fn fan_mail() {
    let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
    let payment_option = client.get_payment_option("").unwrap();
    let name = SecretKey::random().to_hex();
    let (addressee, _) = client
        .addressee_create(&name, payment_option.clone(), None)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let mut door_mat = client
        .doormat_init(addressee.secret_key(), &name)
        .await
        .unwrap();
    // test with nothing posted
    assert!(door_mat.items().is_empty());
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    // send loads of mail
    let items_to_send = 1000;
    let message = Bytes::from("Hello I'm you biggest fan!");
    for i in 0..items_to_send {
        client
            .package_post(addressee.address(), message.clone(), payment_option.clone())
            .await
            .unwrap();
        eprintln!("Posting no {i} at {:?}", SystemTime::now());
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), items_to_send);
    assert_eq!(
        door_mat.items().first().unwrap().payload().unwrap(),
        message
    );
    assert_eq!(door_mat.items().last().unwrap().payload().unwrap(), message);
}
