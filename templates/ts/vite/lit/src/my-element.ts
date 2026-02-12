import { LitElement, css, html } from "lit";
import { customElement, property } from "lit/decorators.js";
import litLogo from "./assets/lit.svg";
import viteLogo from "/vite.svg";

@customElement("my-element")
export class MyElement extends LitElement {
	render() {
		return html``;
	}
	static styles = css``;
}

declare global {
	interface HTMLElementTagNameMap {
		"my-element": MyElement;
	}
}
