use crate::{mock::*, Error, IngamePlayers, NewPlayers};
use crate::{PlayerCount, Tickets};
use frame_support::{assert_err, assert_ok, traits::Currency};
use gafi_primitives::currency::{unit, NativeToken::GAKI};
use gafi_primitives::{
	system_services::SystemPool,
	constant::ID
};
use sp_runtime::AccountId32;
use sp_std::str::FromStr;

const CIRCLE_BLOCK: u64 = (TIME_SERVICE as u64) / SLOT_DURATION;
const UPFRONT_BASIC_ID: ID = [192, 153, 16, 32, 147, 221, 194, 86, 16, 108, 55, 91, 150, 248, 93, 75, 158, 180, 246, 128, 72, 1, 237, 12, 3, 89, 3, 209, 30, 8, 104, 20];
const UPFRONT_MEDIUM_ID: ID = [1, 207, 121, 218, 73, 69, 195, 112, 198, 139, 38, 94, 247, 6, 65, 170, 166, 94, 170, 143, 89, 83, 227, 144, 13, 151, 114, 76, 44, 90, 160, 149];
const UPFRONT_ADVANCE_ID: ID = [143, 31, 159, 208, 129, 106, 49, 10, 121, 208, 224, 196, 191, 96, 90, 84, 76, 38, 12, 86, 23, 64, 188, 222, 177, 117, 234, 70, 245, 49, 40, 144];

fn make_deposit(account: &AccountId32, balance: u128) {
	let _ = pallet_balances::Pallet::<Test>::deposit_creating(account, balance);
}

fn new_account(balance: u128) -> AccountId32 {
	let alice: AccountId32 =
		AccountId32::from_str("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY").unwrap();
	make_deposit(&alice, balance);
	assert_eq!(Balances::free_balance(&alice), balance);
	return alice;
}

fn new_accounts(count: u32, balance: u128) -> Vec<AccountId32> {
	let mut account_vec = Vec::new();
	for i in 0..count {
		let new_account = AccountId32::new([i as u8; 32]);
		make_deposit(&new_account, balance);
		account_vec.push(new_account);
	}
	account_vec
}

#[test]
fn default_services_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		assert_eq!(UpfrontPool::get_service(UPFRONT_BASIC_ID).is_none(), false);
		assert_eq!(UpfrontPool::get_service(UPFRONT_MEDIUM_ID).is_none(), false);
		assert_eq!(UpfrontPool::get_service(UPFRONT_ADVANCE_ID).is_none(), false);
	})
}

#[test]
fn player_join_pool_should_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(10);
		let count_before = PlayerCount::<Test>::get();
		let alice = new_account(1_000_000 * unit(GAKI));
		assert_ok!(UpfrontPool::join(alice.clone(), UPFRONT_BASIC_ID));

		let player = Tickets::<Test>::get(alice);
		assert_ne!(player, None);

		let count_after = PlayerCount::<Test>::get();
		assert_eq!(count_before, count_after - 1);
	});
}

#[test]
fn set_max_player_should_works() {
	ExtBuilder::default().build_and_execute(|| {
		{
			run_to_block(1);
			let max_player = 10;
			assert_ok!(UpfrontPool::set_max_player(Origin::root(), max_player));
			assert_eq!(UpfrontPool::max_player(), max_player, "max_player after set not correct");
		}

		{
			run_to_block(10);
			let max_player = MAX_PLAYER;
			assert_ok!(UpfrontPool::set_max_player(Origin::root(), max_player));
			assert_eq!(UpfrontPool::max_player(), max_player, "max_player after set not correct");
		}

		{
			run_to_block(20);
			let max_player = MAX_PLAYER;
			assert_ok!(UpfrontPool::set_max_player(Origin::root(), max_player));
			assert_eq!(UpfrontPool::max_player(), max_player, "max_player after set not correct");
		}
	})
}

#[test]
fn set_max_player_should_fail() {
	ExtBuilder::default().build_and_execute(|| {
		// bad origin
		{
			run_to_block(10);
			let max_player = MAX_PLAYER + 1;
			assert_err!(
				UpfrontPool::set_max_player(Origin::signed(AccountId32::new([0; 32])), max_player),
				frame_support::error::BadOrigin
			);
		}
	})
}

#[test]
fn should_restrict_max_player() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(10);
		let max_player = 1000u32;
		assert_ok!(UpfrontPool::set_max_player(Origin::root(), max_player));
		let mut count = 0;
		let accounts = new_accounts(max_player, 1_000_000 * unit(GAKI));
		for account in accounts {
			if count == max_player {
				assert_err!(
					UpfrontPool::join(account, UPFRONT_BASIC_ID),
					<Error<Test>>::ExceedMaxPlayer
				);
			} else {
				assert_ok!(UpfrontPool::join(account, UPFRONT_BASIC_ID));
				count = count + 1;
			}
		}
	})
}

#[test]
fn new_player_leave_pool_should_work() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let alice = new_account(1_000_000 * unit(GAKI));
		assert_ok!(UpfrontPool::join(alice.clone(), UPFRONT_BASIC_ID));
		run_to_block(2);
		assert_ok!(UpfrontPool::leave(alice.clone()));
		assert_eq!(Tickets::<Test>::get(alice.clone()), None);
	})
}

#[test]
fn should_move_newplayers_to_ingame() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let alice = new_account(1_000_000 * unit(GAKI));
		assert_ok!(UpfrontPool::join(alice.clone(), UPFRONT_BASIC_ID));

		{
			let new_players_before = NewPlayers::<Test>::get();
			let ingame_players_before = IngamePlayers::<Test>::get();
			assert_eq!(new_players_before.len(), 1);
			assert_eq!(ingame_players_before.len(), 0);
		}

		run_to_block(CIRCLE_BLOCK);
		{
			let new_players_after = NewPlayers::<Test>::get();
			let ingame_players_after = IngamePlayers::<Test>::get();
			assert_eq!(ingame_players_after.len(), 1);
			assert_eq!(new_players_after.len(), 0);
		}
	})
}

#[test]
fn get_player_level_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let alice = new_account(1_000_000 * unit(GAKI));
		assert_ok!(UpfrontPool::join(alice.clone(), UPFRONT_BASIC_ID));

		let level = UpfrontPool::get_player_level(alice.clone());
		assert_eq!(level.is_none(), false);
	})
}

#[test]
fn get_player_service_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let alice = new_account(1_000_000 * unit(GAKI));
		assert_ok!(UpfrontPool::join(alice.clone(), UPFRONT_BASIC_ID));

		let serevice = UpfrontPool::get_player_service(alice.clone());
		assert_eq!(serevice.is_none(), false);
	})
}

#[test]
fn charge_ingame_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let alice = new_account(1_000_000 * unit(GAKI));
		assert_ok!(UpfrontPool::join(alice.clone(), UPFRONT_BASIC_ID));

		run_to_block(CIRCLE_BLOCK + 1); // move to ingame
		let before_balance = Balances::free_balance(&alice);
		let _ = UpfrontPool::charge_ingame();
		let after_balance = Balances::free_balance(&alice);
		let service = UpfrontPool::get_service(UPFRONT_BASIC_ID).unwrap();
		assert_eq!(before_balance, after_balance + service.value);
	})
}
