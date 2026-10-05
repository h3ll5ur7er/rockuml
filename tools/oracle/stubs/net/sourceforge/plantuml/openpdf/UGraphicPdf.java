package net.sourceforge.plantuml.openpdf;

import net.sourceforge.plantuml.klimt.drawing.UGraphic;
import net.sourceforge.plantuml.klimt.font.StringBounder;

public class UGraphicPdf {
	public static UGraphic build(PdfOption option, StringBounder stringBounder) {
		throw new UnsupportedOperationException("PDF output needs OpenPDF, which the oracle build leaves out");
	}
}
