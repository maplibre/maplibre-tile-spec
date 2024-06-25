const express = require('express');
const fs = require('fs');
const path = require('path');
const cors = require('cors');

const port = 8081;
const app = express();
app.use(cors());

const directoryPath = process.argv[2];
if (!directoryPath) {
    console.error('Please provide a directory path as a command-line argument.');
    process.exit(1);
}

app.get('*', (req, res, next) => {
    const filePath = path.join(__dirname, directoryPath, decodeURI(req.path));
    if (req.path.endsWith(".gz")) {
        res.setHeader('Content-Encoding', 'gzip');
        res.sendFile(filePath);
    } else if (fs.existsSync(filePath)) {
        res.sendFile(filePath);
    } else {
        res.status(404).send('File not found');
    }
});

app.listen(port, () => {
    console.log(`Server is running at http://localhost:${port}`);
    console.log(`Serving files from directory: ${directoryPath}`);
});