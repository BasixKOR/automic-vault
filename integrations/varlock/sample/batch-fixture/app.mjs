import 'varlock/auto-load';
import { ENV } from 'varlock/env';

if (ENV.VARLOCK_SAMPLE_FIRST !== 'VARLOCK_SAMPLE_FIRST'
    || ENV.VARLOCK_SAMPLE_SECOND !== 'VARLOCK_SAMPLE_SECOND') {
  throw new Error('Secrets were not resolved as one batch');
}

console.log('Varlock resolved two Secret Names through one helper request.');
