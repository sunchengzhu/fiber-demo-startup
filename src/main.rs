use ckb_hash::blake2b_256;
use ckb_jsonrpc_types::{CellOutput, JsonBytes, OutPoint, Script, ScriptHashType};
use ckb_sdk::constants::SIGHASH_TYPE_HASH;
use ckb_sdk::rpc::ckb_indexer::{Order, SearchKey, SearchKeyFilter};
use ckb_sdk::traits::{DefaultTransactionDependencyProvider, SecpCkbRawKeySigner};
use ckb_sdk::tx_builder::unlock_tx;
use ckb_sdk::unlock::{ScriptUnlocker, SecpSighashUnlocker};
use ckb_sdk::{CkbRpcClient, ScriptId};
use ckb_types::core::TransactionView;
use ckb_types::packed::{Byte, CellInput, CellOutputBuilder, Script as PackedScript, WitnessArgs};
use ckb_types::prelude::*;
use ckb_types::{H256, h256};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

const CKB_RPC_URL: &str = "http://127.0.0.1:8114";

const SOURCE_PRIVATE_KEY: &str = "63d86723e08f0f813a36ce6aa123bb2289d90680ae1e99d4de8cdb334553f24d";

// Key file paths
const BOOTNODE_KEY_FILE: &str = "fiber/ckb-keys/bootnode-key";

const NODE1_KEY_FILE: &str = "fiber/ckb-keys/node1-key";
const NODE2_KEY_FILE: &str = "fiber/ckb-keys/node2-key";
const NODE3_KEY_FILE: &str = "fiber/ckb-keys/node3-key";

const SUDT_CODE_HASH: H256 =
    h256!("0xe1e354d6d643ad42724d40967e334984534e0367405c5ae42a9d7d63d77df419");
const SUDT_ARGS: &str = "c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd";

// 10 billion CKB = 10^9 * 10^8 shannons
const CKB_TRANSFER_AMOUNT: u64 = 1_000_000_000_00000000;
// 10 billion sUDT
const SUDT_TRANSFER_AMOUNT: u128 = 1_000_000_000;
// Minimum cell capacity for sUDT cell (142 CKB)
const MIN_SUDT_CELL_CAPACITY: u64 = 142_00000000;
// Transaction fee
const TX_FEE: u64 = 100000;

#[derive(Debug)]
pub struct LiveCell {
    pub out_point: OutPoint,
    pub output: CellOutput,
    pub output_data: JsonBytes,
}

/// Read private key from file
fn read_private_key(path: &str) -> String {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read private key from {}: {}", path, e));
    content.trim().to_string()
}

fn get_lock_script_from_private_key(private_key_hex: &str) -> Script {
    let secp = Secp256k1::new();
    let private_key_bytes = hex::decode(private_key_hex).expect("Invalid hex string");
    let secret_key = SecretKey::from_slice(&private_key_bytes).expect("Invalid private key");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    let pubkey_bytes = public_key.serialize();
    let pubkey_hash = blake2b_256(&pubkey_bytes);
    let pubkey_hash160: [u8; 20] = pubkey_hash[0..20].try_into().unwrap();

    Script {
        code_hash: SIGHASH_TYPE_HASH.clone(),
        hash_type: ScriptHashType::Type,
        args: JsonBytes::from_vec(pubkey_hash160.to_vec()),
    }
}

fn list_live_cells(client: &CkbRpcClient, private_key_hex: &str) -> Vec<LiveCell> {
    let lock_script = Script {
        code_hash: SIGHASH_TYPE_HASH.clone(),
        hash_type: ScriptHashType::Type,
        args: {
            let secp = Secp256k1::new();
            let secret_key = SecretKey::from_str(private_key_hex).unwrap();
            let public_key = PublicKey::from_secret_key(&secp, &secret_key);
            let pubkey_hash_160: [u8; 20] = blake2b_256(&public_key.serialize())[0..20]
                .try_into()
                .unwrap();

            JsonBytes::from_vec(pubkey_hash_160.to_vec())
        },
    };

    let search_key = SearchKey {
        script: lock_script,
        script_type: ckb_sdk::rpc::ckb_indexer::ScriptType::Lock,
        script_search_mode: Some(ckb_sdk::rpc::ckb_indexer::SearchMode::Exact),
        filter: None,
        with_data: Some(true),
        group_by_transaction: Some(false),
    };

    let mut live_cells = Vec::new();
    let mut cursor = None;

    loop {
        let cells = client
            .get_cells(
                search_key.clone(),
                Order::Asc,
                100u32.into(),
                cursor.clone(),
            )
            .expect("Failed to get cells");

        if cells.objects.is_empty() {
            break;
        }

        for cell in cells.objects {
            live_cells.push(LiveCell {
                out_point: cell.out_point,
                output: cell.output,
                output_data: cell.output_data.unwrap_or_default(),
            });
        }

        cursor = Some(cells.last_cursor);
    }

    live_cells
}

/// Find pure CKB cells (without type script)
fn find_ckb_cells(client: &CkbRpcClient, private_key_hex: &str) -> Vec<LiveCell> {
    let all_cells = list_live_cells(client, private_key_hex);
    all_cells
        .into_iter()
        .filter(|cell| cell.output.type_.is_none())
        .collect()
}

/// Get sUDT type script
fn get_sudt_type_script() -> Script {
    Script {
        code_hash: SUDT_CODE_HASH.clone(),
        hash_type: ScriptHashType::Data,
        args: JsonBytes::from_vec(hex::decode(SUDT_ARGS).unwrap()),
    }
}

/// Find sUDT cells owned by the given private key
fn find_sudt_cells(client: &CkbRpcClient, private_key_hex: &str) -> Vec<LiveCell> {
    let lock_script = get_lock_script_from_private_key(private_key_hex);
    let sudt_type_script = get_sudt_type_script();

    let search_key = SearchKey {
        script: lock_script.clone(),
        script_type: ckb_sdk::rpc::ckb_indexer::ScriptType::Lock,
        script_search_mode: None,
        filter: Some(SearchKeyFilter {
            script: Some(sudt_type_script),
            script_len_range: None,
            output_data: None,
            output_data_filter_mode: None,
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        }),
        with_data: Some(true),
        group_by_transaction: None,
    };

    let mut sudt_cells = Vec::new();
    let mut cursor = None;

    loop {
        let cells = client
            .get_cells(
                search_key.clone(),
                Order::Asc,
                100u32.into(),
                cursor.clone(),
            )
            .expect("Failed to get cells");

        if cells.objects.is_empty() {
            break;
        }

        for cell in cells.objects {
            sudt_cells.push(LiveCell {
                out_point: cell.out_point,
                output: cell.output,
                output_data: cell.output_data.unwrap_or_default(),
            });
        }

        cursor = Some(cells.last_cursor);
    }

    sudt_cells
}

/// Parse sUDT amount from cell data (little-endian u128)
fn parse_sudt_amount(data: &[u8]) -> u128 {
    if data.len() < 16 {
        return 0;
    }
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[0..16]);
    u128::from_le_bytes(bytes)
}

/// Encode amount as sUDT cell data (little-endian u128)
fn encode_sudt_amount(amount: u128) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

/// Get secp256k1 cell dep (from genesis block)
fn get_secp256k1_cell_dep(client: &CkbRpcClient) -> ckb_types::packed::OutPoint {
    let genesis = client.get_block_by_number(0u64.into()).unwrap().unwrap();
    let tx_hash = genesis.transactions[1].hash.clone();
    ckb_types::packed::OutPoint::new_builder()
        .tx_hash(tx_hash.0.pack())
        .index(0u32)
        .build()
}

/// Get sUDT cell dep
fn get_sudt_cell_dep(client: &CkbRpcClient) -> ckb_types::packed::OutPoint {
    let genesis = client.get_block_by_number(0u64.into()).unwrap().unwrap();
    let tx_hash = genesis.transactions[0].hash.clone();
    ckb_types::packed::OutPoint::new_builder()
        .tx_hash(tx_hash.0.pack())
        .index(8u32)
        .build()
}

/// Sign transaction
fn sign_transaction(tx: TransactionView, private_key_hex: &str) -> TransactionView {
    let private_key_bytes = hex::decode(private_key_hex).unwrap();
    let secret_key = secp256k1::SecretKey::from_slice(&private_key_bytes).unwrap();

    let tx_dep_provider = DefaultTransactionDependencyProvider::new(CKB_RPC_URL, 10);

    let signer = SecpCkbRawKeySigner::new_with_secret_keys(vec![secret_key]);
    let script_id = ScriptId::new_type(SIGHASH_TYPE_HASH.clone());
    let unlocker = SecpSighashUnlocker::from(Box::new(signer) as Box<_>);

    let mut unlockers: HashMap<ScriptId, Box<dyn ScriptUnlocker>> = HashMap::new();
    unlockers.insert(script_id, Box::new(unlocker));

    let (signed_tx, _) = unlock_tx(tx, &tx_dep_provider, &unlockers).unwrap();
    signed_tx
}

/// Build packed lock script from private key
fn build_packed_lock_script(private_key_hex: &str) -> PackedScript {
    let lock_script = get_lock_script_from_private_key(private_key_hex);
    PackedScript::new_builder()
        .code_hash(lock_script.code_hash.0.pack())
        .hash_type(Byte::new(lock_script.hash_type as u8))
        .args(lock_script.args.as_bytes().pack())
        .build()
}

/// Build packed sUDT type script
fn build_packed_sudt_type_script() -> PackedScript {
    let sudt_type_script = get_sudt_type_script();
    PackedScript::new_builder()
        .code_hash(sudt_type_script.code_hash.0.pack())
        .hash_type(Byte::new(sudt_type_script.hash_type as u8))
        .args(sudt_type_script.args.as_bytes().pack())
        .build()
}

/// Transfer CKB to multiple recipients
fn transfer_ckb(
    client: &CkbRpcClient,
    from_private_key: &str,
    recipients: &[(&str, u64)], // (private_key, amount)
) -> H256 {
    // Calculate total amount needed
    let total_amount: u64 = recipients.iter().map(|(_, amount)| *amount).sum();
    let total_needed = total_amount + TX_FEE;

    // Find CKB cells
    let ckb_cells = find_ckb_cells(client, from_private_key);

    // Collect enough inputs
    let mut inputs = Vec::new();
    let mut input_capacity: u64 = 0;

    for cell in &ckb_cells {
        inputs.push(
            CellInput::new_builder()
                .previous_output(
                    ckb_types::packed::OutPoint::new_builder()
                        .tx_hash(cell.out_point.tx_hash.0.pack())
                        .index(cell.out_point.index.value() as u32)
                        .build(),
                )
                .build(),
        );
        input_capacity += u64::from(cell.output.capacity);

        if input_capacity >= total_needed {
            break;
        }
    }

    assert!(
        input_capacity >= total_needed,
        "Not enough CKB. Have: {}, Need: {}",
        input_capacity,
        total_needed
    );

    // Build outputs
    let mut outputs = Vec::new();
    let mut outputs_data = Vec::new();

    for (recipient_key, amount) in recipients {
        let lock_script = build_packed_lock_script(recipient_key);
        let output = CellOutputBuilder::default()
            .capacity(ckb_types::core::Capacity::shannons(*amount).pack())
            .lock(lock_script)
            .build();
        outputs.push(output);
        outputs_data.push(ckb_types::packed::Bytes::default());
    }

    // Change output
    let change_amount = input_capacity - total_amount - TX_FEE;
    if change_amount > 0 {
        let change_lock_script = build_packed_lock_script(from_private_key);
        let change_output = CellOutputBuilder::default()
            .capacity(ckb_types::core::Capacity::shannons(change_amount).pack())
            .lock(change_lock_script)
            .build();
        outputs.push(change_output);
        outputs_data.push(ckb_types::packed::Bytes::default());
    }

    // Build transaction
    let mut tx_builder = TransactionView::new_advanced_builder();

    for input in inputs.iter() {
        tx_builder = tx_builder.input(input.clone());
    }

    for output in outputs {
        tx_builder = tx_builder.output(output);
    }

    for data in outputs_data {
        tx_builder = tx_builder.output_data(data);
    }

    // Add witnesses (one for each input)
    for _ in 0..inputs.len() {
        tx_builder = tx_builder.witness(WitnessArgs::default().as_bytes().pack());
    }

    let tx = tx_builder
        .cell_dep(
            ckb_types::packed::CellDep::new_builder()
                .out_point(get_secp256k1_cell_dep(client))
                .dep_type(Byte::new(ckb_types::core::DepType::DepGroup as u8))
                .build(),
        )
        .build();

    // Sign and send
    let tx = sign_transaction(tx, from_private_key);

    let tx_hash = client
        .send_transaction(tx.data().into(), None)
        .expect("Failed to send CKB transfer transaction");

    println!("CKB transfer transaction sent: {:#x}", tx_hash);
    tx_hash
}

/// Transfer sUDT to multiple recipients
fn transfer_sudt(
    client: &CkbRpcClient,
    from_private_key: &str,
    recipients: &[(&str, u128)], // (private_key, sudt_amount)
) -> H256 {
    // Calculate total sUDT amount needed
    let total_sudt_amount: u128 = recipients.iter().map(|(_, amount)| *amount).sum();

    // Find sUDT cells
    let sudt_cells = find_sudt_cells(client, from_private_key);
    assert!(!sudt_cells.is_empty(), "No sUDT cells found");

    // Collect sUDT inputs
    let mut inputs = Vec::new();
    let mut input_sudt_amount: u128 = 0;
    let mut input_capacity: u64 = 0;

    for cell in &sudt_cells {
        inputs.push(
            CellInput::new_builder()
                .previous_output(
                    ckb_types::packed::OutPoint::new_builder()
                        .tx_hash(cell.out_point.tx_hash.0.pack())
                        .index(cell.out_point.index.value() as u32)
                        .build(),
                )
                .build(),
        );
        input_sudt_amount += parse_sudt_amount(cell.output_data.as_bytes());
        input_capacity += u64::from(cell.output.capacity);

        if input_sudt_amount >= total_sudt_amount {
            break;
        }
    }

    assert!(
        input_sudt_amount >= total_sudt_amount,
        "Not enough sUDT. Have: {}, Need: {}",
        input_sudt_amount,
        total_sudt_amount
    );

    // Calculate capacity needed for outputs
    let capacity_per_output = MIN_SUDT_CELL_CAPACITY;
    let total_output_capacity = capacity_per_output * recipients.len() as u64;

    // Check if we need more CKB cells for capacity
    let mut ckb_inputs = Vec::new();
    if input_capacity < total_output_capacity + TX_FEE + MIN_SUDT_CELL_CAPACITY {
        // Need more CKB
        let ckb_cells = find_ckb_cells(client, from_private_key);
        let needed_capacity =
            total_output_capacity + TX_FEE + MIN_SUDT_CELL_CAPACITY - input_capacity;
        let mut collected: u64 = 0;

        for cell in &ckb_cells {
            ckb_inputs.push(
                CellInput::new_builder()
                    .previous_output(
                        ckb_types::packed::OutPoint::new_builder()
                            .tx_hash(cell.out_point.tx_hash.0.pack())
                            .index(cell.out_point.index.value() as u32)
                            .build(),
                    )
                    .build(),
            );
            collected += u64::from(cell.output.capacity);
            input_capacity += u64::from(cell.output.capacity);

            if collected >= needed_capacity {
                break;
            }
        }
    }

    // Build outputs
    let mut outputs = Vec::new();
    let mut outputs_data = Vec::new();
    let sudt_type_script = build_packed_sudt_type_script();

    for (recipient_key, sudt_amount) in recipients {
        let lock_script = build_packed_lock_script(recipient_key);
        let output = CellOutputBuilder::default()
            .capacity(ckb_types::core::Capacity::shannons(capacity_per_output).pack())
            .lock(lock_script)
            .type_(Some(sudt_type_script.clone()).pack())
            .build();
        outputs.push(output);
        outputs_data.push(encode_sudt_amount(*sudt_amount).pack());
    }

    // Change sUDT output
    let change_sudt_amount = input_sudt_amount - total_sudt_amount;
    let change_capacity = input_capacity - total_output_capacity - TX_FEE;

    if change_sudt_amount > 0 {
        let change_lock_script = build_packed_lock_script(from_private_key);
        let change_output = CellOutputBuilder::default()
            .capacity(ckb_types::core::Capacity::shannons(MIN_SUDT_CELL_CAPACITY).pack())
            .lock(change_lock_script.clone())
            .type_(Some(sudt_type_script.clone()).pack())
            .build();
        outputs.push(change_output);
        outputs_data.push(encode_sudt_amount(change_sudt_amount).pack());

        // Pure CKB change if any remaining
        let remaining_capacity = change_capacity - MIN_SUDT_CELL_CAPACITY;
        if remaining_capacity > 61_00000000 {
            let ckb_change_output = CellOutputBuilder::default()
                .capacity(ckb_types::core::Capacity::shannons(remaining_capacity).pack())
                .lock(change_lock_script)
                .build();
            outputs.push(ckb_change_output);
            outputs_data.push(ckb_types::packed::Bytes::default());
        }
    } else if change_capacity > 61_00000000 {
        // Only CKB change
        let change_lock_script = build_packed_lock_script(from_private_key);
        let change_output = CellOutputBuilder::default()
            .capacity(ckb_types::core::Capacity::shannons(change_capacity).pack())
            .lock(change_lock_script)
            .build();
        outputs.push(change_output);
        outputs_data.push(ckb_types::packed::Bytes::default());
    }

    // Build transaction
    let mut tx_builder = TransactionView::new_advanced_builder();

    // Add sUDT inputs first
    for input in inputs.iter() {
        tx_builder = tx_builder.input(input.clone());
    }

    // Add CKB inputs
    for input in ckb_inputs.iter() {
        tx_builder = tx_builder.input(input.clone());
    }

    for output in outputs {
        tx_builder = tx_builder.output(output);
    }

    for data in outputs_data {
        tx_builder = tx_builder.output_data(data);
    }

    // Add witnesses
    let total_inputs = inputs.len() + ckb_inputs.len();
    for _ in 0..total_inputs {
        tx_builder = tx_builder.witness(WitnessArgs::default().as_bytes().pack());
    }

    let tx = tx_builder
        .cell_dep(
            ckb_types::packed::CellDep::new_builder()
                .out_point(get_secp256k1_cell_dep(client))
                .dep_type(Byte::new(ckb_types::core::DepType::DepGroup as u8))
                .build(),
        )
        .cell_dep(
            ckb_types::packed::CellDep::new_builder()
                .out_point(get_sudt_cell_dep(client))
                .dep_type(Byte::new(ckb_types::core::DepType::Code as u8))
                .build(),
        )
        .build();

    // Sign and send
    let tx = sign_transaction(tx, from_private_key);

    let tx_hash = client
        .send_transaction(tx.data().into(), None)
        .expect("Failed to send sUDT transfer transaction");

    println!("sUDT transfer transaction sent: {:#x}", tx_hash);
    tx_hash
}

fn main() {
    let client = CkbRpcClient::new(CKB_RPC_URL);

    println!("=== Fiber Demo Startup: Transfer CKB and sUDT to nodes ===\n");

    // Read private keys from files
    let source_key = SOURCE_PRIVATE_KEY.to_string();
    let bootnode_key = read_private_key(BOOTNODE_KEY_FILE);
    let node1_key = read_private_key(NODE1_KEY_FILE);
    let node2_key = read_private_key(NODE2_KEY_FILE);
    let node3_key = read_private_key(NODE3_KEY_FILE);

    println!("Loaded private keys from files:");
    println!("  Source: {}", source_key);
    println!("  Bootnode:  {}", BOOTNODE_KEY_FILE);
    println!("  Node1:  {}", NODE1_KEY_FILE);
    println!("  Node2:  {}", NODE2_KEY_FILE);
    println!("  Node3:  {}", NODE3_KEY_FILE);
    println!();

    // Print recipient addresses
    println!("Target accounts:");
    for (i, key) in [&bootnode_key, &node1_key, &node2_key, &node3_key]
        .iter()
        .enumerate()
    {
        let lock_script = get_lock_script_from_private_key(key);
        println!(
            "  Node{}: args = 0x{}",
            i + 1,
            hex::encode(lock_script.args.as_bytes())
        );
    }
    println!();

    // Check source account balance
    let ckb_cells = find_ckb_cells(&client, &source_key);
    let total_ckb: u64 = ckb_cells.iter().map(|c| u64::from(c.output.capacity)).sum();
    println!("Source CKB balance: {} CKB", total_ckb / 100000000);

    let sudt_cells = find_sudt_cells(&client, &source_key);
    let total_sudt: u128 = sudt_cells
        .iter()
        .map(|c| parse_sudt_amount(c.output_data.as_bytes()))
        .sum();
    println!("Source sUDT balance: {}", total_sudt);
    println!();

    // Transfer CKB (10 billion CKB to each node)
    println!(
        "Transferring {} CKB to each node...",
        CKB_TRANSFER_AMOUNT / 100000000
    );
    let ckb_recipients: Vec<(&str, u64)> = vec![
        (&bootnode_key, CKB_TRANSFER_AMOUNT),
        (&node1_key, CKB_TRANSFER_AMOUNT),
        (&node2_key, CKB_TRANSFER_AMOUNT),
        (&node3_key, CKB_TRANSFER_AMOUNT),
    ];

    let ckb_tx_hash = transfer_ckb(&client, &source_key, &ckb_recipients);
    println!("CKB transfer complete: {:#x}\n", ckb_tx_hash);

    // Wait for transaction to be committed
    println!("Waiting for CKB transaction to be committed...");
    std::thread::sleep(std::time::Duration::from_secs(10));

    // Transfer sUDT (10 billion sUDT to each node)
    println!("Transferring {} sUDT to each node...", SUDT_TRANSFER_AMOUNT);
    let sudt_recipients: Vec<(&str, u128)> = vec![
        (&node1_key, SUDT_TRANSFER_AMOUNT),
        (&node2_key, SUDT_TRANSFER_AMOUNT),
        (&node3_key, SUDT_TRANSFER_AMOUNT),
    ];

    let sudt_tx_hash = transfer_sudt(&client, &source_key, &sudt_recipients);
    println!("sUDT transfer complete: {:#x}\n", sudt_tx_hash);

    println!("=== All transfers complete! ===");
}
