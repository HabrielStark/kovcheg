# Coq formal verification build file for ARK
# Usage: make all  (compile proofs)
#        make clean

COQFILES := PUF_CT.v Masking.v DSL_Soundness.v Kyber_CT.v Voter_Skew.v

all: $(COQFILES:.v=.vo)

%.vo: %.v
	coqc $<

clean:
	rm -f *.vo *.glob 

# Compile and fully check proofs, emitting coqchk log (CI target)
proof: all
	@echo "Running coqchk…"
	coqchk -silent $(COQFILES:.v=.vo) | tee formal_proof.log 