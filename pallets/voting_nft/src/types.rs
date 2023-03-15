use codec::{Decode, Encode};
use frame_support::{pallet_prelude::MaxEncodedLen, traits::Get, BoundedVec};
// use frame_support::BoundedVec;
use scale_info::TypeInfo;
// use sp_core::Get;

/// contains the tokens staked/allocated for each vote and the proposal hash
#[derive(Encode, Decode, TypeInfo, Default, Debug, MaxEncodedLen, Clone, PartialEq)]
#[scale_info(skip_type_params(T))]
pub struct VoteAllocated<Hash, Conviction, CollectionId, ItemId> {
	pub proposal: Hash,
	pub vote: Votes<Conviction, CollectionId, ItemId>,
}

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEq, Clone, Default)]
#[scale_info(skip_type_params(M))]
pub struct Proposal<AccountId, M: Get<u32>> {
	pub title: BoundedVec<u32, M>,
	pub author: AccountId,
	pub about: BoundedVec<u32, M>,
	pub department: BoundedVec<u32, M>,
	// expiry: BlockNumber,
}

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEq, Clone, Default)]
#[scale_info(skip_type_params(T))]
pub struct Votes<Conviction, CollectionId, ItemId> {
	pub vote: bool,
	pub conviction: Conviction,
	pub collection: CollectionId,
	pub item: ItemId,
}

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Clone, PartialEq, Ord, Eq, PartialOrd)]
// #[scale_info(skip_type_params(T))]
pub struct VoteNumbers<Hash> {
	pub number_of_votes: u128,
	pub proposal: Hash,
}

#[derive(
	Encode,
	Decode,
	Copy,
	Clone,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Debug,
	Default,
	TypeInfo,
)]
pub enum Conviction {
	#[default]
	None,
	Locked1x,
	Locked2x,
}

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum Phase {
	ProposalPhase,
	VotingPhase,
	Expired,
	Accepted,
	Aborted,
}
