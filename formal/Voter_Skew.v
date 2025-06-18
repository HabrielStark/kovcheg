(* ARK Formal Proof: Voter-Skew Safety – ensures quorum accuracy *)
Require Import Coq.Arith.Arith.
Require Import Coq.Lists.List.
Import ListNotations.

(* Assume voter outputs are booleans *)
Definition Vote := bool.

(* Skew: difference between #true and #false votes *)
Definition skew (lst : list Vote) : nat :=
  let trues := length (filter (fun v => v) lst) in
  let falses := length (filter (fun v => negb v) lst) in
  Nat.abs (Nat.sub trues falses).

(* Safety predicate: skew ≤ 1 for robust decision *)
Definition skew_safe (lst : list Vote) : Prop := skew lst <= 1.

(* Balanced list: equal #true and #false votes *)
Definition balanced (lst : list Vote) : Prop :=
  length (filter (fun v => v) lst) = length (filter (fun v => negb v) lst).

(* Lemma: adding balanced pair preserves safety *)
Lemma skew_add_pair : forall lst,
  skew_safe lst -> skew_safe (true :: false :: lst).
Proof.
  intros lst Hsafe.
  unfold skew_safe, skew.
  simpl.
  (* trues +1, falses +1 so skew unchanged *)
  unfold skew in Hsafe.
  (* trivial rewriting: abs(trues+1 - falses-1)=abs(trues-falses) *)
  remember (length (filter (fun v : bool => v) lst)) as t.
  remember (length (filter (fun v : bool => negb v) lst)) as f.
  simpl.
  rewrite Nat.add_comm.
  simpl.
  rewrite Nat.add_comm with (n:=1) (m:=t).
  rewrite Nat.add_comm with (n:=1) (m:=f).
  simpl.
  unfold skew in Hsafe.
  rewrite Heqt in Hsafe. rewrite Heqf in Hsafe.
  exact Hsafe.
Qed.

(* Main theorem: balanced list ⇒ skew_safe *)
Theorem voter_skew_safe : forall lst,
  balanced lst -> skew_safe lst.
Proof.
  intros lst Hbal.
  unfold balanced in Hbal.
  unfold skew_safe, skew.
  rewrite Hbal.
  rewrite Nat.sub_diag.
  simpl.
  apply Nat.le_0_l.
Qed. 