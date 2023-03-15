#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

mod types;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::nonfungible_v2::{self, Inspect},
			BalanceStatus::Reserved,
			Currency, EnsureOriginWithArg, ReservableCurrency,
		},
	};
	use frame_system::pallet_prelude::*;
	// use sp_std::prelude::*;

	/// type of counting votes
	pub type VoteCount = u128;

	/// Number of votes stored in a bounded vec
	pub type VoteInNumbers<T> =
		BoundedVec<types::VoteNumbers<<T as frame_system::Config>::Hash>, ConstU32<100>>;
	/// The proposal id type. This is a hash of the proposal which is all we need to make it unique
	pub type ProposalID<T> = <T as frame_system::Config>::Hash;
	/// The type for checking Balance of an accountId
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	/// This is the item id of each nft
	pub type ItemIdOf<T> =
		<<T as Config>::Inspect as Inspect<<T as frame_system::Config>::AccountId>>::ItemId;
	/// This is the collection id of every item or item id of an nft
	// pub type CollectionIdOf<T> =
	// 	<<T as Config>::Inspect as Inspect<<T as frame_system::Config>::AccountId>>::CollectionId;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	const PROPOSAL_PHASE_LENGTH: u32 = 1000;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The inspect mechanism for checking authenticity of nfts
		type Inspect: Inspect<Self::AccountId>;

		#[pallet::constant]
		type MaxLength: Get<u32>;

		// type NonFungibles: nonfungibles_v2::Inspect<Self::AccountId>
		// 	+ nonfungibles_v2::Transfer<Self::AccountId>;

		// type NonFungibles: nonfungibles_v2

		/// The currency mechanism, used for paying for reserves.
		type Currency: ReservableCurrency<Self::AccountId>;
	}

	#[pallet::type_value]
	pub fn PhaseStart<T: Config>() -> types::Phase {
		types::Phase::ProposalPhase
	}

	#[pallet::storage]
	#[pallet::getter(fn registered_voters_with_votes)]
	pub type RegisteredVotersWithVotes<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, VoteInNumbers<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_proposals)]
	// The proposal is just the hash of its text which is all that's needed to make it unique
	pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, ProposalID<T>, VoteCount>;

	#[pallet::storage]
	#[pallet::getter(fn get_current_phase)]
	pub type InPhase<T: Config> = StorageValue<_, types::Phase, ValueQuery, PhaseStart<T>>;

	#[pallet::storage] //initialize counter with 0
	#[pallet::getter(fn total_proposals)]
	pub type TotalProposals<T: Config> = StorageValue<_, u32, ValueQuery, GetDefault>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_creators)]
	pub type ProposalCreators<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		VoterRegistered { who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		AccountAlreadyRegistered,
		WrongPhaseError,
		NotRegisteredError,
		ProposalMultipleByOneError,
		ProposalLimitReachedError,
		ProposalDuplicateError,
		ProposalSubmitted,
		VoteForSameProposalError,
		// Other errors
		OverflowError,
		ConversionError,
		VoteForNonExistentProposalError,
		VoteAllocationZeroError,
		VoteWithInsufficientFundsError,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn register_voter(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				RegisteredVotersWithVotes::<T>::get(&who).is_none(),
				Error::<T>::AccountAlreadyRegistered
			);

			RegisteredVotersWithVotes::<T>::insert(&who, <VoteInNumbers<T> as Default>::default());

			Self::deposit_event(Event::VoterRegistered { who });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn submit_proposal(
			origin: OriginFor<T>,
			title: BoundedVec<u32, T::MaxLength>,
			about: BoundedVec<u32, T::MaxLength>,
			department: BoundedVec<u32, T::MaxLength>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check we are in the proposal Phase
			ensure!(Self::is_proposal_phase(), Error::<T>::WrongPhaseError);
			//check if it's from a registered voter
			ensure!(Self::is_registered(&who), Error::<T>::NotRegisteredError);
			//check if the user has not already created a proposal
			ensure!(!Self::created_proposal_already(&who), Error::<T>::ProposalMultipleByOneError);

			
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// Helper functions

		pub fn is_registered(who: &T::AccountId) -> bool {
			RegisteredVotersWithVotes::<T>::contains_key(who)
		}

		pub fn created_proposal_already(who: &T::AccountId) -> bool {
			ProposalCreators::<T>::contains_key(who)
		}

		pub fn proposal_exists(proposal: &ProposalID<T>) -> bool {
			// there is no API to just check for the first value of the key, essentially treating it
			// as a single StorageMap with (key1, value) pairs TODO: write own
			// DoubleToSingleStorageMap trait to provide this functionality for DoubleMap
			// Single StorageMap approach is just simple contains_key check
			Proposals::<T>::contains_key(proposal)
		}

		// count up to get the next proposal ID value
		pub fn get_total() -> u32 {
			TotalProposals::<T>::get() //make into 0 on None
		}
		// pub fn increase_total() {
		// 	//we dont need to check for overflow bc our maxProposal is below u32::max
		// 	TotalProposals::<T>::mutate(|x| *x += 1);
		// }

		pub fn is_proposal_phase() -> bool {
			matches!(InPhase::<T>::get(), types::Phase::ProposalPhase)
		}
	}
}
