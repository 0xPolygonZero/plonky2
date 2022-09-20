DOCNAME=zkevm

all: pdf

.PHONY: clean

quick:
	pdflatex $(DOCNAME).tex

pdf:
	pdflatex $(DOCNAME).tex
	bibtex $(DOCNAME).aux
	pdflatex $(DOCNAME).tex
	pdflatex $(DOCNAME).tex

view: pdf
	open $(DOCNAME).pdf

clean:
	rm -f *.blg *.bbl *.aux *.log
