package org.teavm.jso.dom.html;
public interface HTMLDocument extends org.teavm.jso.dom.xml.Document {
	static HTMLDocument current() { throw new UnsupportedOperationException(); }
	HTMLElement getElementById(String id);
	HTMLElement createElement(String name);
}
