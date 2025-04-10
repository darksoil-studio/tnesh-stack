import { mdiAlertCircleOutline } from '@mdi/js';
import '@shoelace-style/shoelace/dist/components/icon/icon.js';
import '@shoelace-style/shoelace/dist/components/tooltip/tooltip.js';
import { LitElement, css, html } from 'lit';
import { customElement, property } from 'lit/decorators.js';

import { wrapPathInSvg } from '../icon.js';
import { sharedStyles } from '../shared-styles.js';

@customElement('display-error')
export class DisplayError extends LitElement {
	@property({ attribute: 'tooltip' })
	tooltip: boolean = false;

	@property()
	headline: string | undefined;

	@property()
	error!: any;

	@property({ attribute: 'icon-size' })
	iconSize: string | undefined;

	get _iconSize() {
		if (this.iconSize) return this.iconSize;
		if (this.tooltip !== false) return '32px';
		return '64px';
	}

	renderIcon() {
		return html`
			<sl-icon
				style="color: red; height: ${this._iconSize}; width: ${this._iconSize};"
				src="${wrapPathInSvg(mdiAlertCircleOutline)}"
			></sl-icon>
		`;
	}

	renderFull() {
		return html` <div class="column center-content" style="flex: 1; gap: 8px">
			${this.renderIcon()}
			<div style="max-width: 500px; text-align: center" class="column">
				${this.headline
					? html` <span style="margin-bottom: 8px">${this.headline} </span>`
					: html``}
				<span class="placeholder"
					>${typeof this.error === 'object' && 'message' in this.error
						? this.error.message
						: this.error}
				</span>
			</div>
		</div>`;
	}

	renderTooltip() {
		return html`
			<sl-tooltip hoist .content=${this.headline ? this.headline : this.error}>
				${this.renderIcon()}</sl-tooltip
			>
		`;
	}

	render() {
		if (this.tooltip !== false) return this.renderTooltip();
		return this.renderFull();
	}

	static styles = [
		sharedStyles,
		css`
			:host {
				display: flex;
			}
		`,
	];
}
