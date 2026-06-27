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

## TRON wallet fingerprint API

Wallet fingerprinting identifies the target wallet, its direct sender wallets, and
its direct receiver wallets from historical `address_relationships` flow data. The
API combines exchange attribution, entity labels, contract metadata, transaction
risk, DeFi behavior, bridge behavior, activity timing, token diversity, and
counterparty concentration.

Run the graph API:

```bash
cargo run --bin tron_graph_api
```

Query a wallet fingerprint:

```bash
curl "http://127.0.0.1:3000/api/tron/wallet/<TRON_WALLET_ADDRESS>/fingerprint?window_days=90&top_counterparties=25&max_events=20000"
```

The response includes:

- `identity`: best current label for the requested wallet, using exchange/entity/contract/profile data.
- `fingerprint_label` and `wallet_type`: behavior class such as exchange deposit funnel, collector, distributor, DeFi swapper, bridge user, service hub, or retail wallet.
- `flows`: inbound/outbound transfer counts, unique sender and receiver counts, raw volume totals, and observed transaction risk.
- `behavior`: active hours, active days, burst score, average transaction interval, token diversity, contract/swap/bridge/exchange ratios, and counterparty concentration.
- `senders`: direct wallets that funded the target wallet, each with identity, relationship label, tokens, volume, first/last seen, risk, and share of wallet activity.
- `receivers`: direct wallets that received funds from the target wallet, with the same fingerprint details.
- `risk_flags`: compact AML flags for high risk transactions, exchange-heavy flow, burst activity, concentration, fan-in/fan-out patterns, swap-heavy activity, and bridge-heavy activity.

Schema hooks are also created in ClickHouse:

- `wallet_fingerprints` stores wallet-level snapshots for search and dashboards.
- `wallet_counterparty_fingerprints` stores sender/receiver relationship snapshots for fast investigation views.
