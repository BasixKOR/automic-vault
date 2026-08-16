import 'varlock/auto-load';

if (!process.env.VARLOCK_SAMPLE_SECRET) throw new Error('Secret was not resolved');
console.log('Varlock received the approved Secret Value.');
