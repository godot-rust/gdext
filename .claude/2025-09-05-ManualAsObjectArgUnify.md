i have the AsObjectArg<T> trait, which I'd like to merge into the AsArg trait with impls AsArg<Gd<T>> + AsArg<Option<Gd<T>>. i am aware there might be coherence problems (overlapping impls in stable rust) 
  due to ToGodot::Pass=ByRef for Gd<T>. ultrathink if it's possible to work around that, possibly considering a third Pass type (besides ByRef/ByValue). come up with a comprehensive and detailed plan, and a 
  step-wise migration where you can quickly test the feasibility without first needing to rework everything.
