mod sequence_proposal;
mod view_proposal;

pub(crate) use sequence_proposal::SequenceProposal;
pub(crate) use view_proposal::ViewProposal;

/*

ViewProposal
Output = (HashSet<ViewProposal>, Certificate)
SequenceProposal = (HashSet<ViewDecision>, Certificate)

Map Decisions(Resolution) to Set<Change>

SequenceProposal
SequenceDecision

Resolution


*/
