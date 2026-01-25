use autonomi::pointer::PointerTarget;
use autonomi::{Bytes, Client, GraphEntryAddress, Pointer, SecretKey};
use postem::addressee::PostemName;
use postem::{ConnectionType, PostemClient, PostemError};
use std::thread;
use std::time::SystemTime;

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

    // after adding a pointer (ie not valid package)
    let mut route = client
        .route_get(PostemName::create(&name).unwrap(), false)
        .await
        .unwrap();
    let next_location = client.location_get_available(&mut route).await.unwrap();
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
    let mut route = client
        .route_get(PostemName::create(&name).unwrap(), false)
        .await
        .unwrap();
    let next_location = client.location_get_available(&mut route).await.unwrap();
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

// stress test adding a lot of pack:ges and thousands of non-packages will take 25 mins+ to run
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
    let items_to_send = 100;
    let message = Bytes::from("Hello I'm you biggest fan!");
    for i in 0..items_to_send {
        client
            .package_post(addressee.address(), message.clone(), payment_option.clone())
            .await
            .unwrap();
        eprintln!("Posting no {i} at {:?}", SystemTime::now());
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), items_to_send);
    assert_eq!(
        door_mat.items().first().unwrap().payload().unwrap(),
        message
    );
    assert_eq!(door_mat.items().last().unwrap().payload().unwrap(), message);
    // send load of non packages
    for i in 0..items_to_send {
        let mut route = client
            .route_get(PostemName::create(&name).unwrap(), false)
            .await
            .unwrap();
        let next_location = client.location_get_available(&mut route).await.unwrap();
        let pointer = Pointer::new(
            &next_location,
            0,
            PointerTarget::GraphEntryAddress(GraphEntryAddress::new(
                SecretKey::random().public_key(),
            )),
        );
        let autonomi_client = Client::init_local().await.unwrap();
        autonomi_client
            .pointer_put(pointer, payment_option.clone())
            .await
            .unwrap();
        eprintln!("Posting non package {i} at {:?}", SystemTime::now());
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    assert_eq!(door_mat.items().len(), items_to_send);
    assert_eq!(
        door_mat.items().first().unwrap().payload().unwrap(),
        message
    );
    // send lots more mail
    let message2 = Bytes::from("Hiya I'm you biggest bigest ever fan!");
    for i in 0..items_to_send {
        client
            .package_post(
                addressee.address(),
                message2.clone(),
                payment_option.clone(),
            )
            .await
            .unwrap();
        eprintln!("Posting no {i} at {:?}", SystemTime::now());
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), items_to_send * 2);
    assert_eq!(
        door_mat.items().first().unwrap().payload().unwrap(),
        message
    );
    assert_eq!(
        door_mat.items().last().unwrap().payload().unwrap(),
        message2
    );
}

// test sending loads of packages is quickly as possible using mutiple threads as just looping
// async call seem to be in the region of 0.5-1 sec apart and I want to test if it retries if
// an location on the route has been used before a post_package call completes
// no quicker then doing async...maybe explicit test in package for this
#[tokio::test]
async fn heavy_traffic() {
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
    let mut handles = vec![];
    for i in 0..10 {
        let client = client.clone();
        let addressee = addressee.clone();
        let payment_option = payment_option.clone();
        handles.push(thread::spawn(move || async move {
            let message = Bytes::from(format!("I'm posting it first! {}", i));
            client
                .package_post(addressee.address(), message.clone(), payment_option.clone())
                .await
                .unwrap();
            eprintln!("Posting no {i} at {:?}", SystemTime::now());
        }));
    }
    for handle in handles {
        handle.join().unwrap().await;
    }
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), 10);
    eprintln!("{:?}", door_mat.items());
}

// tried to do all the object cloning before the thread were spawned but it makes no practical
// difference...is it even running concurrently??
#[tokio::test]
async fn heavy_traffic_mk_ii() {
    let packages_to_post = 8;
    let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
    let payment_option = client.get_payment_option("").unwrap();
    let name = SecretKey::random().to_hex();
    let (addressee, _) = client
        .addressee_create(&name, payment_option.clone(), None)
        .await
        .unwrap();
    let mut door_mat = client
        .doormat_init(addressee.secret_key(), &name)
        .await
        .unwrap();
    // test with nothing posted
    assert!(door_mat.items().is_empty());
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, false);
    let mut closures = Vec::with_capacity(packages_to_post);
    for i in 0..packages_to_post {
        let (client, payment_option, name) =
            (client.clone(), payment_option.clone(), addressee.address());
        let message = Bytes::from(format!("I'm posting it first! {}", i));
        closures.push(move || async move {
            client
                .package_post(name, message, payment_option)
                .await
                .unwrap();
        });
    }
    let mut handles = Vec::with_capacity(packages_to_post);
    for closure in closures {
        handles.push(thread::spawn(|| closure()));
        eprintln!("Spawned at {:?}", SystemTime::now());
    }
    for handle in handles {
        handle.join().unwrap().await;
        eprintln!("Completd at {:?}", SystemTime::now());
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    eprintln!("All Completd at {:?}", SystemTime::now());
    let you_have_got_mail = client.doormat_update(&mut door_mat).await.unwrap();
    assert_eq!(you_have_got_mail, true);
    assert_eq!(door_mat.items().len(), packages_to_post);
    eprintln!("{:?}", door_mat.items());
}
