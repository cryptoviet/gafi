use frame_support::dispatch::DispatchResult;
use sp_std::vec::Vec;

use crate::constant::ID;

pub trait Name<AccountId> {
	fn set_name(origin: AccountId, asset_id: ID, name: Vec<u8>) -> DispatchResult;
	fn clear_name(origin: AccountId,asset_id: ID) -> DispatchResult;
	fn kill_name(origin: AccountId,asset_id: ID) -> DispatchResult;
}
