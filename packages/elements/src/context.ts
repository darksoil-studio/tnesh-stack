import { AppClient } from '@holochain/client';
import { createContext } from '@lit/context';

export const appClientContext = createContext<AppClient>(
	'tnesh/appClientContext',
);
