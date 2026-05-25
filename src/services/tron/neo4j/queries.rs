pub fn address_graph_query(depth: u32) -> String {
    format!(
        "
        MATCH path =
        (a:Wallet)-[*1..{}]-(b)

        WHERE a.address = $address

        RETURN path
        LIMIT 500
        ",
        depth
    )
}
