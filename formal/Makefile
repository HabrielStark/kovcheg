# Coq formal verification build file for ARK
# Usage: make all  (compile proofs)
#        make clean

COQFILES := PUF_CT.v Masking.v DSL_Soundness.v

all: $(COQFILES:.v=.vo)

%.vo: %.v
	coqc $<

clean:
	rm -f *.vo *.glob 