import 'varlock/auto-load';
import { ENV } from 'varlock/env';

const first = ENV.VARLOCK_SAMPLE_SECRET;
const second = ENV.VARLOCK_SAMPLE_SECRET;

if (!first || !second || first !== second) throw new Error('Secret was not resolved twice');
console.log('Varlock received the approved Secret Value twice.');
