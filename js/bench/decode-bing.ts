import glob from 'glob';
import { execSync } from 'child_process';

const searchPattern = '../test/expected/bing/**/*.mlt';

glob(searchPattern, (err, files) => {
    if (err) {
        console.error('Error:', err);
        return;
    }
    files.forEach((file) => {
        console.log()
        console.log(file);
        execSync(`node dist/bench/decode-mlt.js ${file}`,{stdio: 'inherit'});
        const mvt = file.replace(/\.mlt$/, '.mvt').replace('expected', 'fixtures');
        console.log()
        console.log(mvt);
        execSync(`node dist/bench/decode-mvt.js ${mvt}`,{stdio: 'inherit'});
    });
});
