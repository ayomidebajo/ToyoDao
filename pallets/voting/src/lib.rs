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
		traits::{Currency, LockableCurrency, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;

	pub type VoteWeight<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	pub type TotalTokensStakedFromVotes<T> =
		BoundedVec<types::VoteAllocated<T, ProposalID<T>, types::Conviction>, ConstU32<100>>;

	pub type VoteCount = u128;

	pub type VoteInNumbers<T> =
		BoundedVec<types::VoteNumbers<<T as frame_system::Config>::Hash>, ConstU32<100>>;

	pub type ProposalID<T> = <T as frame_system::Config>::Hash;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;
	}

	#[pallet::type_value]
	pub fn PhaseStart<T: Config>() -> types::Phase {
		types::Phase::ProposalPhase
	}

	#[pallet::type_value]
	pub fn CounterStart<T: Config>() -> u32 {
		0u32
	}

	#[pallet::storage]
	#[pallet::getter(fn registered_voters_with_votes)]
	pub type RegisteredVotersWithVotes<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, VoteInNumbers<T>>;

	#[pallet::storage]
	// The proposal is just the hash of its text which is all that's needed to make it unique
	pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, ProposalID<T>, VoteCount>;

	#[pallet::storage]
	pub type InPhase<T: Config> = StorageValue<_, types::Phase, ValueQuery, PhaseStart<T>>;

	#[pallet::storage] //initialize counter with 0
	pub type TotalProposals<T: Config> = StorageValue<_, u32, ValueQuery, CounterStart<T>>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_creators)]
	pub type ProposalCreators<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		VoterRegistered {
			who: T::AccountId,
		},
		ProposalSubmitted {
			proposal: T::Hash,
			who: T::AccountId,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		WrongPhaseError,
		NotRegisteredError,
		ProposalMultipleByOneError,
		ProposalLimitReachedError,
		ProposalDuplicateError,
		ProposalSubmitted,
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
			// T::RegistrationOrigin::ensure_origin(origin)?;
			RegisteredVotersWithVotes::<T>::insert(&who, <VoteInNumbers<T> as Default>::default());
			Self::deposit_event(Event::VoterRegistered { who });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn submit_proposal(origin: OriginFor<T>, text: T::Hash) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// check we are in the proposal Phase
			ensure!(Self::is_proposal_phase(), Error::<T>::WrongPhaseError);
			//check if it's from a registered voter
			ensure!(Self::is_registered(&who), Error::<T>::NotRegisteredError);
			//check if the user has not already created a proposal
			ensure!(!Self::created_proposal_already(&who), Error::<T>::ProposalMultipleByOneError);

			// check that the limit of total proposals has not been reached
			ensure!(Self::get_total() < 100, Error::<T>::ProposalLimitReachedError);
			// check for duplicate proposals
			ensure!(!Self::proposal_exists(&text), Error::<T>::ProposalDuplicateError);

			//create proposal
			ProposalCreators::<T>::insert(&who, ());
			Proposals::<T>::insert(text, 0u128);
			//increment counter
			Self::increase_total();

			//emit event
			Self::deposit_event(Event::ProposalSubmitted { proposal: text, who });

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
		pub fn increase_total() {
			//we dont need to check for overflow bc our maxProposal is below u32::max
			TotalProposals::<T>::mutate(|x| *x += 1);
		}

		pub fn is_proposal_phase() -> bool {
			matches!(InPhase::<T>::get(), types::Phase::ProposalPhase)
		}
	}
}
