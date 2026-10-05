package org.teavm.jso;
public @interface JSBody { String[] params() default {}; String script(); }
