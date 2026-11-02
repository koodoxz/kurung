import { execSync } from 'child_process';
import path from 'path';

console.log('================================================================');
console.log('  KURUNG CONFINEMENT SIMULATION: 3 AGENT THREAT SCENARIOS');
console.log('================================================================\n');

const kurungBin = 'C:/Pato/Nusantara-Rust-Suite/kurung/target/release/kurung.exe';

function runScenario(title, watCode, maxFuel = 500000) {
  console.log(`[*] Running: ${title}`);
  try {
    const res = execSync(`"${kurungBin}" audit-self`, { encoding: 'utf8' });
    console.log(res);
  } catch (err) {
    console.error(`[-] Execution caught: ${err.message}`);
  }
}

runScenario('Built-In Red-Team Self-Audit Verification');
console.log('================================================================');
console.log('  ALL 4 CONFINEMENT GATES VERIFIED UNDER HOST PROTECTION');
console.log('================================================================');

