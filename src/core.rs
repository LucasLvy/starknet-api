#[cfg(test)]
#[path = "core_test.rs"]
mod core_test;

use std::fmt::Debug;

use derive_more::Display;
use once_cell::sync::Lazy;
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use starknet_types_core::felt::Felt;
use starknet_types_core::hash::{Pedersen, StarkHash as StarkHashTrait};

use crate::crypto::PublicKey;
use crate::serde_utils::{BytesAsHex, PrefixedBytesAsHex};
use crate::transaction::{Calldata, ContractAddressSalt};
use crate::{impl_from_through_intermediate, StarknetApiError};

/// A chain id.
#[derive(Clone, Debug, Display, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
pub struct ChainId(pub String);

impl ChainId {
    pub fn as_hex(&self) -> String {
        format!("0x{}", hex::encode(&self.0))
    }
}

/// The address of a contract, used for example in [StateDiff](`crate::state::StateDiff`),
/// [DeclareTransaction](`crate::transaction::DeclareTransaction`), and
/// [BlockHeader](`crate::block::BlockHeader`).

// The block hash table is stored in address 0x1,
// this is a special address that is not used for contracts.
pub const BLOCK_HASH_TABLE_ADDRESS: ContractAddress = ContractAddress(PatriciaKey(Felt::ONE));
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    derive_more::Deref,
)]
pub struct ContractAddress(pub PatriciaKey);

impl From<ContractAddress> for Felt {
    fn from(contract_address: ContractAddress) -> Felt {
        contract_address.0.to_felt()
    }
}

impl From<u128> for ContractAddress {
    fn from(val: u128) -> Self {
        ContractAddress(PatriciaKey::from(val))
    }
}

impl_from_through_intermediate!(u128, ContractAddress, u8, u16, u32, u64);

/// The maximal size of storage var.
pub const MAX_STORAGE_ITEM_SIZE: u16 = 256;
/// The prefix used in the calculation of a contract address.
pub const CONTRACT_ADDRESS_PREFIX: &[u8] = b"STARKNET_CONTRACT_ADDRESS";
/// The size of the contract address domain.
pub static CONTRACT_ADDRESS_DOMAIN_SIZE: Felt = Felt::from_raw_const(PATRICIA_KEY_UPPER_BOUND);
/// The address upper bound; it is defined to be congruent with the storage var address upper bound.
pub static L2_ADDRESS_UPPER_BOUND: Lazy<Felt> =
    Lazy::new(|| CONTRACT_ADDRESS_DOMAIN_SIZE - Felt::from(MAX_STORAGE_ITEM_SIZE));

impl TryFrom<Felt> for ContractAddress {
    type Error = StarknetApiError;
    fn try_from(hash: Felt) -> Result<Self, Self::Error> {
        Ok(Self(PatriciaKey::try_from(hash)?))
    }
}

// TODO: Add a hash_function as a parameter
pub fn calculate_contract_address(
    salt: ContractAddressSalt,
    class_hash: ClassHash,
    constructor_calldata: &Calldata,
    deployer_address: ContractAddress,
) -> Result<ContractAddress, StarknetApiError> {
    let constructor_calldata_hash = Pedersen::hash_array(&constructor_calldata.0);
    let address: Felt = (&(Pedersen::hash_array(&[
        Felt::from_bytes_be_slice(CONTRACT_ADDRESS_PREFIX),
        deployer_address.to_felt(),
        salt.0,
        class_hash.0,
        constructor_calldata_hash,
    ])
    .to_biguint()
        % L2_ADDRESS_UPPER_BOUND.to_biguint()))
        .into();

    ContractAddress::try_from(address)
}

/// The hash of a ContractClass.
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    Display,
    derive_more::Deref,
)]
pub struct ClassHash(pub Felt);

/// The hash of a compiled ContractClass.
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    Display,
)]
pub struct CompiledClassHash(pub Felt);

/// A general type for nonces.
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    derive_more::Deref,
)]
pub struct Nonce(pub Felt);

impl Nonce {
    pub fn try_increment(&self) -> Result<Self, StarknetApiError> {
        // Check if an overflow occurred during increment.
        let new_nonce = self.0 + Felt::ONE;
        if new_nonce == Felt::ZERO {
            Err(StarknetApiError::OutOfRange { string: format!("{:?}", self) })
        } else {
            Ok(Self(new_nonce))
        }
    }
}

/// The selector of an [EntryPoint](`crate::deprecated_contract_class::EntryPoint`).
#[derive(
    Debug, Copy, Clone, Default, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord,
)]
pub struct EntryPointSelector(pub Felt);

/// The root of the global state at a [Block](`crate::block::Block`)
/// and [StateUpdate](`crate::state::StateUpdate`).
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    Eq,
    PartialEq,
    Hash,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    Display,
)]
pub struct GlobalRoot(pub Felt);

/// A key for nodes of a Patricia tree.
// Invariant: key is in range.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Default,
    Hash,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    derive_more:: Deref,
)]
pub struct PatriciaKey(Felt);

/// 2**251 in the Mont representation of the felt type.
pub const PATRICIA_KEY_UPPER_BOUND: [u64; 4] =
    [0x7fffea55af00450, 0xfffffffffffb7c00, 0x9987fff, 0xffffffeb9bf00021];

impl PatriciaKey {
    pub fn as_felt(&self) -> &Felt {
        &self.0
    }

    pub fn to_felt(&self) -> Felt {
        self.0
    }
}

impl From<u128> for PatriciaKey {
    fn from(val: u128) -> Self {
        PatriciaKey::try_from(Felt::from(val)).expect("Failed to convert u128 to PatriciaKey.")
    }
}

impl_from_through_intermediate!(u128, PatriciaKey, u8, u16, u32, u64);

impl TryFrom<Felt> for PatriciaKey {
    type Error = StarknetApiError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        if value < CONTRACT_ADDRESS_DOMAIN_SIZE {
            return Ok(PatriciaKey(value));
        }
        Err(StarknetApiError::OutOfRange {
            string: Felt::from_raw_const(PATRICIA_KEY_UPPER_BOUND).to_string(),
        })
    }
}

impl Debug for PatriciaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PatriciaKey").field(&self.0).finish()
    }
}

/// A utility macro to create a [`PatriciaKey`] from a hex string / unsigned integer representation.
#[cfg(any(feature = "testing", test))]
#[macro_export]
macro_rules! patricia_key {
    ($s:expr) => {
        PatriciaKey::try_from(starknet_types_core::felt::Felt::from($s)).unwrap()
    };
}

/// A utility macro to create a [`ClassHash`] from a hex string / unsigned integer representation.
#[cfg(any(feature = "testing", test))]
#[macro_export]
macro_rules! class_hash {
    ($s:expr) => {
        ClassHash(starknet_types_core::felt::Felt::from($s))
    };
}
/// A utility macro to create a [`ContractAddress`] from a hex string / unsigned integer
/// representation.
#[cfg(any(feature = "testing", test))]
#[macro_export]
macro_rules! contract_address {
    ($s:expr) => {
        ContractAddress(patricia_key!($s))
    };
}

/// An Ethereum address.
#[derive(
    Debug, Copy, Clone, Default, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord,
)]
#[serde(try_from = "PrefixedBytesAsHex<20_usize>", into = "PrefixedBytesAsHex<20_usize>")]
pub struct EthAddress(pub H160);

impl TryFrom<Felt> for EthAddress {
    type Error = StarknetApiError;
    fn try_from(felt: Felt) -> Result<Self, Self::Error> {
        const COMPLIMENT_OF_H160: usize = std::mem::size_of::<Felt>() - H160::len_bytes();
        let bytes = felt.to_bytes_be();
        let (rest, h160_bytes) = bytes.split_at(COMPLIMENT_OF_H160);
        if rest != [0u8; COMPLIMENT_OF_H160] {
            return Err(StarknetApiError::OutOfRange { string: felt.to_string() });
        }

        Ok(EthAddress(H160::from_slice(h160_bytes)))
    }
}

impl From<EthAddress> for Felt {
    fn from(value: EthAddress) -> Self {
        let mut bytes = [0u8; 32];
        // Padding H160 with zeros to 32 bytes (big endian)
        bytes[12..32].copy_from_slice(value.0.as_bytes());
        Felt::from_bytes_be(&bytes)
    }
}

impl TryFrom<PrefixedBytesAsHex<20_usize>> for EthAddress {
    type Error = StarknetApiError;
    fn try_from(val: PrefixedBytesAsHex<20_usize>) -> Result<Self, Self::Error> {
        Ok(EthAddress(H160::from_slice(&val.0)))
    }
}

impl From<EthAddress> for PrefixedBytesAsHex<20_usize> {
    fn from(felt: EthAddress) -> Self {
        BytesAsHex(felt.0.to_fixed_bytes())
    }
}

/// A public key of a sequencer.
#[derive(
    Debug, Copy, Clone, Default, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord,
)]
pub struct SequencerPublicKey(pub PublicKey);
