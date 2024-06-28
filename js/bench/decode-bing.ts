import glob from 'glob';
import { execSync } from 'child_process';

const searchPattern = '../test/expected/bing/**/*.mlt';

glob(searchPattern, (err, files) => {
    if (err) {
        console.error('Error:', err);
        return;
    }
    for (const file of files) {
        console.log()
        execSync(`node dist/bench/decode-mlt.js ${file} 100`,{stdio: 'inherit'});
        const mvt = file.replace(/\.mlt$/, '.mvt').replace('expected', 'fixtures');
        console.log()
        execSync(`node dist/bench/decode-mvt.js ${mvt} 100`,{stdio: 'inherit'});
    }
});
