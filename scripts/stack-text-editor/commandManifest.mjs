import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export async function persistCommandRecord(output, commands, command) {
  commands.push(command);
  await writeFile(join(output, 'commands.json'), JSON.stringify(commands, null, 2));
}
