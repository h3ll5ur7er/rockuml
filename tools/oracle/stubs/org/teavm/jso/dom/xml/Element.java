package org.teavm.jso.dom.xml;
public interface Element extends org.teavm.jso.JSObject {
	void setAttribute(String name, String value);
	Element appendChild(Element child);
	void setTextContent(String text);
}
