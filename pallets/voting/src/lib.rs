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
	use crate::types::{Conviction, Votes};

	use super::*;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::traits::{CheckedAdd, Hash, IntegerSquareRoot},
		traits::{Currency, LockIdentifier, LockableCurrency, ReservableCurrency, WithdrawReasons},
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

	const PROPOSAL_PHASE_LENGTH: u32 = 1000;

	const LOCK_ID: LockIdentifier = *b"example "; //

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

		#[pallet::constant]
		type MaxLength: Get<u32>;
	}

	#[pallet::type_value]
	pub fn PhaseStart<T: Config>() -> types::Phase {
		types::Phase::ProposalPhase
	}

	// #[pallet::type_value]
	// pub fn CounterStart<T: Config>() -> u32 {
	// 	0u32
	// }

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
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		VoterRegistered {
			who: T::AccountId,
		},
		ProposalSubmitted {
			proposal: T::Hash,
			who: T::AccountId,
		},
		VoteSubmitted {
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
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn register_voter(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

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

			// check if proposal created by user is not a duplicate, by checking if the proposal is
			// running and also if it's been created by that origin (who)

			// check that the limit of total proposals has not been reached
			ensure!(Self::get_total() < 100, Error::<T>::ProposalLimitReachedError);

			let new_proposal: types::Proposal<T::AccountId, T::MaxLength> =
				types::Proposal { title, about, author: who.clone(), department };

			let proposal_hash = T::Hashing::hash_of(&new_proposal);

			// check for duplicate proposals
			ensure!(!Self::proposal_exists(&proposal_hash), Error::<T>::ProposalDuplicateError);

			//create proposal
			ProposalCreators::<T>::insert(&who, ());
			Proposals::<T>::insert(proposal_hash, 0u128);

			//increment total amount of proposals counter
			Self::total_proposals();

			//emit event
			Self::deposit_event(Event::ProposalSubmitted { proposal: proposal_hash, who });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		// Extrinsic to submit a vote
		pub fn submit_vote(
			origin: OriginFor<T>,
			vote: Votes<Conviction>,
			proposal_id: ProposalID<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check if in voting phase
			ensure!(!Self::is_proposal_phase(), Error::<T>::WrongPhaseError);
			//check if person is registered to vote
			ensure!(Self::is_registered(&who), Error::<T>::NotRegisteredError);
			//check if the person has already voted
			ensure!(!Self::has_voted(&who, &proposal_id), Error::<T>::VoteForSameProposalError);
			// check if the proposal exists
			ensure!(
				Self::proposal_exists(&proposal_id),
				Error::<T>::VoteForNonExistentProposalError
			);

			Proposals::<T>::mutate(proposal_id, |maybe_value| {
				if let Some(value) = maybe_value {
					if vote.vote {
						let plus = value.saturating_add(1);
						*value = plus;
					} else {
						let minus = value.saturating_sub(1);
						*value = minus;
					}
				}
			});

			// TODO: calculate the vote weight and write logic based on vote weight, i.e if the weight is large, the origin will likely be posted to that department

			// RegisteredVotersWithVotes::<T>::mutate()
			//lock funds
			// T::Currency::set_lock(LOCK_ID, &who, total_tokens, WithdrawReasons::all());
			Self::deposit_event(Event::VoteSubmitted { who });

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

		pub fn calculate_and_check_votes(
			id: &ProposalID<T>,
			vote_weight: &VoteWeight<T>,
			zero_balance: BalanceOf<T>,
		) -> Result<types::VoteNumbers<T::Hash>, Error<T>> {
			// check if proposal id exists
			ensure!(Self::proposal_exists(id), Error::<T>::VoteForNonExistentProposalError);

			//check if the vote was not zero
			ensure!(vote_weight > &zero_balance, Error::<T>::VoteAllocationZeroError);

			// get actual number of votes
			let rt = vote_weight.integer_sqrt(); //square root of voteWeight, rounding down
			let no_of_votes =
				TryInto::<u128>::try_into(rt).map_err(|_| Error::<T>::ConversionError)?;

			let vote = types::VoteNumbers { number_of_votes: no_of_votes, proposal: *id };
			Ok(vote)
		}

		pub fn has_voted(who: &T::AccountId, proposal_id: &ProposalID<T>) -> bool {
			let mut res = false;

			let b_vec = RegisteredVotersWithVotes::<T>::get(who).expect("error, voter not found");

			for i in b_vec {
				if i.proposal.encode().as_slice() == proposal_id.encode().as_slice() {
					res = true;
				}
			}
			res
		}

		pub fn has_enough_funds(who: &T::AccountId, total_tokens: &BalanceOf<T>) -> DispatchResult {
			let bal = &T::Currency::free_balance(&who);
			ensure!(total_tokens <= bal, Error::<T>::VoteWithInsufficientFundsError);
			Ok(())
		}
	}
}
