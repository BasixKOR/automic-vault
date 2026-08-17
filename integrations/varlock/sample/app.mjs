import 'varlock/auto-load';

const first = process.env.VARLOCK_SAMPLE_SECRET_FIRST;
const second = process.env.VARLOCK_SAMPLE_SECRET_SECOND;

if (!first || !second || first !== second) throw new Error('Secret was not resolved twice');
console.log('Varlock received the approved Secret Value twice.');
