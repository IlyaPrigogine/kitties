#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::{
		sp_runtime::traits::{Hash, Zero},
		traits::{Currency, Randomness, tokens::ExistenceRequirement},
		transactional,
		// dispatch::{DispatchResultWithPostInfo, DispatchResult},
		// pallet_prelude::*
	};
	use sp_io::hashing::blake2_128;
	use scale_info::TypeInfo;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	//  Struct for holding Kitty Information
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

	// Set Gender type in Kitty Struct
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Gender {
		Male,
		Female,
	}


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	// #[pallet::generate_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the Kitties pallet.
		type Currency: Currency<Self::AccountId>;

		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		type MaxKittyOwned: Get<u32>;

	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		/// Handles arithemtic overflow when incrementing the Kitty counter.
		KittyCntOverflow,
		/// An account cannot own more Kitties than `MaxKittyCount`.
		ExceedMaxKittyOwned,
		/// Buyer cannot be the owner.
		BuyerIsKittyOwner,
		/// Cannot transfer a kitty to its owner.
		TransferToSelf,
		/// Handles checking whether the Kitty exists.
		KittyNotExist,
		/// Handles checking that the Kitty is owned by the account transferring, buying or setting a price for it.
		NotKittyOwner,
		/// Ensures the Kitty is for sale.
		KittyNotForSale,
		/// Ensures that the buying price is greater than the asking price.
		KittyBidPriceTooLow,
		/// Ensures that an account has enough funds to purchase a Kitty.
		NotEnoughBalance,
	}

	// Events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new Kitty was sucessfully created. \[sender, kitty_id\]
		Created(T::AccountId, T::Hash),
		/// Kitty price was sucessfully set. \[sender, kitty_id, new_price\]
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		/// A Kitty was sucessfully transferred. \[from, to, kitty_id\]
		Transferred(T::AccountId, T::AccountId, T::Hash),
		/// A Kitty was sucessfully bought. \[buyer, seller, kitty_id, bid_price\]
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
	}

	// Storage
	#[pallet::storage]
	#[pallet::getter(fn kitty_cnt)]
	pub(super) type KittyCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub(super) type Kitties<T: Config> = StorageMap<_,Twox64Concat, T::Hash, Kitty<T>>;

	#[pallet::storage]
	#[pallet::getter(fn kitties_owned)]
	/// Keeps track of what accounts own what Kitty.
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Hash, T::MaxKittyOwned>,
		ValueQuery
	>;

	// TODO Part IV: Our pallet's genesis configuration.

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(100)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let kitty_id = Self::mint(&sender, None, None)?;

			// Logging to the console
			log::info!("🎈😺 A kitty is born with ID ➡ {:?}.", kitty_id);
			// Deposit our "Created" event.
			Self::deposit_event(Event::Created(sender, kitty_id));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: T::Hash,
			new_price: Option<BalanceOf<T>>
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);

			let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			kitty.price = new_price.clone();
			<Kitties<T>>::insert(&kitty_id, kitty);

			Self::deposit_event(Event::PriceSet(sender,kitty_id, new_price));

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			kitty_id: T::Hash
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			ensure!(Self::is_kitty_owner(&kitty_id, &from)?,<Error<T>>::NotKittyOwner);

			ensure!(from != to, <Error<T>>::TransferToSelf);

			let to_owned = <KittiesOwned<T>>::get(&to);
			ensure!((to_owned.len() as u32) <T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			Self::transfer_kitty_to(&kitty_id, &to)?;
			Self::deposit_event(Event::Transferred(from,to,kitty_id));

			Ok(())
		}

		// TODO Part III: buy_kitty

		// TODO Part III: breed_kitty
	}

	// TODO Parts II: helper function for Kitty struct

	impl<T: Config> Pallet<T> {
		fn gen_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female,
			}
		}

		// TODO Part III: helper functions for dispatchable functions

		fn gen_dna() -> [u8; 16] {
			let payload = (
				T::KittyRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::block_number(),
				);

			payload.using_encoded(blake2_128)
		}

		// Helper to mint a Kitty.
		pub fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			gender: Option<Gender>,
		) -> Result<T::Hash, Error<T>> {
			let kitty = Kitty::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				gender: gender.unwrap_or_else(Self::gen_gender),
				owner: owner.clone(),
			};

			let kitty_id = T::Hashing::hash_of(&kitty);

			// Performs this operation first as it may fail
			let new_cnt = Self::kitty_cnt().checked_add(1)
				.ok_or(<Error<T>>::KittyCntOverflow)?;

			// Performs this operation first because as it may fail
			<KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| {
				kitty_vec.try_push(kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			<Kitties<T>>::insert(kitty_id, kitty);
			<KittyCnt<T>>::put(new_cnt);
			Ok(kitty_id)
		}

		pub fn is_kitty_owner(kitty_id: &T::Hash, acct: &T::AccountId) -> Result<bool, Error<T>> {
			match Self::kitties(kitty_id) {
				Some(kitty) => Ok(kitty.owner == *acct),
				None => Err(<Error<T>>::KittyNotExist)
			}
		}

		#[transactional]
		pub fn transfer_kitty_to(
			kitty_id: &T::Hash,
			to: &T::AccountId,
		) -> Result<(), Error<T>> {
			let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			let prev_owner = kitty.owner.clone();

			// Remove `kitty_id` from the KittyOwned vector of `prev_kitty_owner`
			<KittiesOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *kitty_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			}).map_err(|_| <Error<T>>::KittyNotExist)?;

			// Update the kitty owner
			kitty.owner = to.clone();
			// Reset the ask price so the kitty is not for sale until `set_price()` is called
			// by the current owner.
			kitty.price = None;

			<Kitties<T>>::insert(kitty_id, kitty);

			<KittiesOwned<T>>::try_mutate(to, |vec| {
				vec.try_push(*kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Ok(())
		}

	}
}
