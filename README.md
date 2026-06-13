## Now BTC is availabe (Global API)

### How to run docker to store BTC data on Clickhouse DB

1. First start You docker on Windows
2. CD to project directory ( cd ./Dockerized_Services )

3. Run codes

```bash
docker compose build --no-cache
```

```bash
APP_MODE=btc docker compose up -d
```

4. See Log is Running Now ...

```bash
docker compose logs -f
```

### Intract with Clickhouse Database

```bash
docker exec -it clickhouse clickhouse-client
```

And then intract with database:
```sql
show databases;
use btc_db;
show tables;
select * from wallet_info;
```

### Docker Neo4j section

    docker network create blockchain-net
    docker compose up -d clickhouse neo4j
    cargo run --bin arz_axum_for_services
    cargo run --bin tron_export_wallet_graph -- TEPSrSYPDSQ7yXpMFPq91Fb1QEWpMkRGfn 5 500

And then we are going to see this
http://localhost:7474/browser/
with
neo4j/password

### use this to visualize 
    
    cargo run --bin tron_graph_api
    curl -X POST "http://localhost:3000/tron/wallet/<TRON_WALLET_ADDRESS>/neo4j/import?depth=3&limit=500"

## New way of Web section

    cargo run --bin tron_graph_api
    http://127.0.0.1:3000/
