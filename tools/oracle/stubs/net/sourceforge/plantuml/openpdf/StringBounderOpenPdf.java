package net.sourceforge.plantuml.openpdf;

import net.sourceforge.plantuml.FileFormat;
import net.sourceforge.plantuml.klimt.font.StringBounder;
import net.sourceforge.plantuml.klimt.font.UFont;
import net.sourceforge.plantuml.klimt.geom.XDimension2D;

public class StringBounderOpenPdf implements StringBounder {
	public XDimension2D calculateDimension(UFont font, String text) {
		throw new UnsupportedOperationException("PDF output needs OpenPDF, which the oracle build leaves out");
	}

	public FileFormat getFileFormat() {
		return FileFormat.PDF;
	}
}
