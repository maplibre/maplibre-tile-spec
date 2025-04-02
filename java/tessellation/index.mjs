import express from "express";
import bodyParser from "body-parser";
import earcut from "earcut";


/* Quick and dirty workaround to us the js version of earcut for pre-tessellating polygons in the java converter
*  since the Java version of earcut seems to have minor issue */
const app = express();
app.use(bodyParser.json({ limit: '50mb' }));
app.use(bodyParser.urlencoded({ limit: '50mb', extended: true }));
app.use(express.json());

app.post('/tessellate', (req, res) => {
    const data = req.body;
    const {vertices, holes} = data;

    const indices = earcut(vertices, holes, 2);
    const response = {indices};
    res.json(response);
});

const port = 3000;
app.listen(port, () => {
    console.log(`Server is running at http://localhost:${port}`);
});
