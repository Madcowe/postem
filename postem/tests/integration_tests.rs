use autonomi::{Bytes, SecretKey};
use postem::{ConnectionType, PostemClient};

#[tokio::test]
async fn test_sending_and_receiving() {
    let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
    let payment_option = client.get_payment_option("").unwrap();
    let name = SecretKey::random().to_hex();
    let addressee = client
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
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), 1);
    eprintln!("{:?}", door_mat.items().first().unwrap());
    assert_eq!(
        door_mat
            .items()
            .first()
            .expect("Should be something on doormat")
            .payload()
            .expect("message should have a payload"),
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
