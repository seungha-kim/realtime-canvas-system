use system::euclid::default::Point2D;
use system::{
    ClientFollowerDocument, Color, DocumentCommand, DocumentMutation, DocumentReadable,
    Materialize, ServerLeaderDocument,
};

#[test]
fn it_should_materialize_oval() {
    let mut server = ServerLeaderDocument::new();
    let mut client = ClientFollowerDocument::new(server.snapshot());

    let tx_result = client
        .handle_command(DocumentCommand::CreateOval {
            r_v: 20.0,
            r_h: 30.0,
            pos: Point2D::new(40.0, 50.0),
            fill_color: Color {
                r: 50,
                g: 50,
                b: 50,
            },
        })
        .unwrap();

    let tx = server.process_transaction(tx_result.transaction).unwrap();
    let oval_object_id = match &tx.items[0] {
        DocumentMutation::CreateObject(object_id, _) => object_id.clone(),
        _ => panic!("unexpected transaction"),
    };

    let oval_material_from_server = server.materialize_oval(&oval_object_id).unwrap();

    let oval_material_from_client = client.materialize_oval(&oval_object_id).unwrap();
    assert_eq!(
        format!("{:?}", oval_material_from_server),
        format!("{:?}", oval_material_from_client)
    );

    let tx_result = client
        .handle_command(DocumentCommand::CreateOval {
            r_v: 20.0,
            r_h: 30.0,
            pos: Point2D::new(40.0, 50.0),
            fill_color: Color {
                r: 50,
                g: 50,
                b: 50,
            },
        })
        .unwrap();
    server.process_transaction(tx_result.transaction).unwrap();

    let document_material_from_server = server.materialize_document();
    assert_eq!(document_material_from_server.children.len(), 2);

    let document_material_from_client = client.materialize_document();
    assert_eq!(document_material_from_client.children.len(), 2);
}
