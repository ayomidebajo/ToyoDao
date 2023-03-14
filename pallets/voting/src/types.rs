use codec::{Decode, Encode};
use frame_support::pallet_prelude::MaxEncodedLen;
use scale_info::TypeInfo;

/// contains the tokens staked/allocated for each vote and the proposal hash
#[derive(Encode, Decode, TypeInfo, Default, Debug, MaxEncodedLen, Clone, PartialEq)]
#[scale_info(skip_type_params(T))]
pub struct VoteAllocated<Currency, Hash, Conviction> {
	pub allocation: Currency,
	pub proposal: Hash,
	pub vote: Votes<Conviction>,
}

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEq, Clone, Default)]
#[scale_info(skip_type_params(T))]
pub struct Votes<Conviction> {
	pub vote: bool,
	pub conviction: Conviction,
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
}
