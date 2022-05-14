// This file is part of Gafi Network.

// Copyright (C) 2021-2022 CryptoViet.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
#[cfg(feature = "std")]
use frame_support::serde::{Deserialize, Serialize};
use frame_system::pallet_prelude::*;
use gafi_primitives::cache::Cache;
use pallet_timestamp::{self as timestamp};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Wrap data with the timestamp at the time when data insert into Cache
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub(super) struct WrapData<Data> {
		pub data: Data,
		pub timestamp: u128,
	}

	impl<Data> WrapData<Data> {
		fn new(data: Data, timestamp: u128) -> Self {
			WrapData { data, timestamp }
		}
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, Copy, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub(super) enum Flag {
		Left,
		Right,
	}

	impl Default for Flag {
		fn default() -> Self {
			Flag::Left
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Data contain the data that need to be storage to cache
		type Data: Parameter + MaxEncodedLen + Copy + TypeInfo;

		/// The Action is the name of action use to query
		type Action: Parameter + MaxEncodedLen + Copy + TypeInfo;
	}

	//** Genesis Conguration **//
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub clean_time: u128,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				clean_time: 3_600_000u128,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<CleanTime<T>>::put(self.clean_time);
			let _now: u128 = <timestamp::Pallet<T>>::get()
				.try_into()
				.ok()
				.unwrap_or_default();
			<MarkTime<T>>::put(_now);
		}
	}

	//** STORAGE **//
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Holding the flag(Left or Right) to support Cache in insert and clean
	#[pallet::type_value]
	pub(super) fn DefaultDataFlag() -> Flag {
		Flag::Left
	}
	#[pallet::storage]
	pub(super) type DataFlag<T: Config> = StorageValue<_, Flag, ValueQuery, DefaultDataFlag>;

	/// Holding the data that insert in Cache by keys AccountId and Action
	#[pallet::storage]
	pub(super) type DataLeft<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, T::Action, WrapData<T::Data>>;

	/// Holding the data that insert in Cache by keys AccountId and Action
	#[pallet::storage]
	pub(super) type DataRight<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, T::Action, WrapData<T::Data>>;

	/// Holding the mark time clean cache
	/// The default value is at the time chain launched
	#[pallet::type_value]
	pub fn DefaultMarkTime<T: Config>() -> u128 {
		<timestamp::Pallet<T>>::get().try_into().ok().unwrap()
	}
	#[pallet::storage]
	#[pallet::getter(fn mark_time)]
	pub type MarkTime<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMarkTime<T>>;

	/// Honding the specific period of time to clean cache
	/// The default value is 1 hours
	#[pallet::type_value]
	pub fn DefaultCleanTime() -> u128 {
		3_600_000u128
	}
	#[pallet::storage]
	#[pallet::getter(fn clean_time)]
	pub type CleanTime<T: Config> = StorageValue<_, u128, ValueQuery, DefaultCleanTime>;

	//** HOOKS **//
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block_number: BlockNumberFor<T>) {
			let _now: u128 = Self::get_timestamp();
			if _now - Self::mark_time() >= Self::clean_time() {
				if DataFlag::<T>::get() == Flag::Left {
					<DataRight<T>>::remove_all(None);
					DataFlag::<T>::put(Flag::Right);
				} else {
					<DataLeft<T>>::remove_all(None);
					DataFlag::<T>::put(Flag::Left);
				}
				MarkTime::<T>::put(_now);
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		pub fn get_timestamp() -> u128 {
			let _now: u128 = <timestamp::Pallet<T>>::get()
				.try_into()
				.ok()
				.unwrap_or_else(|| u128::default());
			_now
		}
	}

	impl<T: Config> Cache<T::AccountId, T::Action, T::Data> for Pallet<T> {

		/// Store data to cache by AccountId and action name
		///
		/// Parameters:
		/// - `id`: data owner
		/// - `action`: The action name
		///	- `data`: The data to store in the cache
		/// 
		/// Weight: `O(1)`
		fn insert(id: &T::AccountId, action: T::Action, data: T::Data) {
			let _now = Self::get_timestamp();
			let wrap_data = WrapData::new(data, _now);
			if DataFlag::<T>::get() == Flag::Left {
				DataLeft::<T>::insert(id, action, wrap_data);
			} else {
				DataRight::<T>::insert(id, action, wrap_data);
			}
		}

		/// Get valid data in cache by AccountId and action name
		///
		/// Parameters:
		/// - `id`: data owner
		///	- `action`: action name
		/// 
		/// Weight: `O(1)`
		fn get(id: &T::AccountId, action: T::Action) -> Option<T::Data> {
			let get_wrap_data = || -> Option<WrapData<T::Data>> {
				if let Some(data) = DataLeft::<T>::get(id, action) {
					return Some(data);
				} else if let Some(data) = DataRight::<T>::get(id, action) {
					return Some(data);
				}
				None
			};

			if let Some(wrap_data) = get_wrap_data() {
				let _now = Self::get_timestamp();
				if _now - wrap_data.timestamp < CleanTime::<T>::get() {
					return Some(wrap_data.data);
				}
			}
			None
		}
	}
}
