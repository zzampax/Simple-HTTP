# Simple RUST custom HTTP Blog Page
![Rust](https://external-content.duckduckgo.com/iu/?u=https%3A%2F%2Frustacean.net%2Fmore-crabby-things%2Frustdocs.png&f=1&nofb=1&ipt=e2106bf2df223e5190325f534d2420ae79a4a430b67bdfda06918721d3786544&ipo=images)
## Description

This is a completly custom made HTTP server, it doesn't use any external library to handle the HTTP requests, it's all made from scratch.
This is **not** a production ready server, it's just a simple project to learn how to handle HTTP requests in RUST.

## Project Structure

The libraries used in this project are:
- **tokio**: To handle the asyncronous tasks.
  - **tokio::net::TcpListener**: To listen for incoming connections.
  - **tokio::io::AsyncReadExt**: To read the incoming data.
  - **tokio::io::AsyncWriteExt**: To write the response.
  - **tokio::fs**: To read the files from the disk.
- *uuid*: To generate the UUIDs.
- *sha256*: To hash the passwords.
- *base64*: To encode and decode the base64 strings.
- *urlencoding*: To encode and decode the URL strings.
- *json*: To parse and create JSON objects.
- *rusqlite*: To handle the SQLite database.
- *colored*: To color the output in the terminal

The following is the project tree structure:
```
src/
├── db.rs
├── http
│   ├── handle_get.rs
│   ├── handle_post.rs
│   ├── mod.rs
│   └── token.rs
├── main.rs
└── multipart
    ├── binary.rs
    └── mod.rs
```
+ **db.rs**: Contains the functions to interact with the SQLite database.
+ **http**: Contains the functions to handle the HTTP requests.
  - **handle_get.rs**: Contains the functions to handle the GET requests.
  - **handle_post.rs**: Contains the functions to handle the POST requests.
  - **mod.rs**: Publishes the functions to handle the HTTP requests.
  - **token.rs**: Contains the functions to handle the authentication tokens.
+ **multipart**: Contains the functions to handle the multipart requests.
  - **binary.rs**: Contains the functions to handle the binary data sent in the multipart requests.
  - **mod.rs**: Handles the multipart requests and calls the functions to handle the binary data.
+ **main.rs**: Contains the main function to start the server (TcpListener).

## Database Structure

The database has the following tables:
- **users**: Contains the users' data.
  - **email**: The email of the user
  - **password**: The hashed password of the user
  - Primary key: ***email***
- **tokens**: Contains the tokens' data.
  - **token**: The token of the user
  - **email**: The email of the user
  - Primary key: ***email***
  - Foreign key: ***email*** references ***users(email)*** on delete cascade
- **posts**: Contains the posts' data.
  - **post_id**: The UUID of the post
  - **email**: The email of the user that created the post
  - **title**: The title of the post
  - **content**: The content of the post
  - **image**: The UUID of the image of the post
  - **datetime**: The date and time of the post
  - Primary key: ***post_id***
  - Foreign key: ***email*** references ***users(email)*** on delete cascade
- **comments**: Contains the comments' data.
  - **comment_id**: The UUID of the comment
  - **post_id**: The UUID of the post
  - **email**: The email of the user that created the comment
  - **content**: The content of the comment
  - **datetime**: The date and time of the comment
  - Primary key: ***comment_id***
  - Foreign key: ***post_id*** references ***posts(post_id)*** on delete cascade
  - Foreign key: ***email*** references ***users(email)*** on delete cascade
![Database Structure](dbstructure.png)

## How to run

To run the server, you need to have the RUST installed in your machine, you can install it by following the instructions in the [official website](https://www.rust-lang.org/tools/install).

After installing the RUST, you can clone this repository and run the following command in the root folder of the project:
```bash
cargo run
```

This will compile and run the server, you can access it by opening the browser and going to the address `http://<any>:8080`.

The `public` folder contains the files that will be served by the server, you can add more files to this folder and access them by going to the address `http://localhost:3000/<file_name>`.
If the extension of the file is in `["png", "jpg", "jpeg", "gif", "ico"]` the server will serve the file as a binary data, otherwise it will serve the file as a text data.

Posts' images are stored in the `public/images` folder as `asset-<uuid>.<ext>`, where `<uuid>` is the UUID of the post and `<ext>` is the extension of the image.

The server will create a SQLite database in the root folder of the project called `blog.db`, you can use the `sqlite3` command to access the database and see the tables and data.
I personally recommend adding the following script to the `.sqliterc` file in your home folder to make the output more readable:
```sql
.mode column
.headers on
.width 15 25 15
```

## Worth mentioning
+ Semi-Dynamic BUFFERS: The server uses a dynamic utf8 buffer to store the incoming data, although it is actually extracted from the socket via a fixed-size buffer. This is done to avoid complications with asyncronous tasks.
The static buffer is filled with the incoming data and then the dynamic buffer is filled with the static buffer, this way the dynamic buffer will always have the incoming data and the static buffer will be filled with the next incoming data.
This cycle is repeated until the end of the incoming data is reached, thus the static buffer will not be filled to the end and the dynamic buffer will have the exact size of the incoming data.
```rust
let mut complete_buffer: Vec<u8> = Vec::new();
let mut buffer: [u8; 16384] = [0; 16384];
loop {
    let bytes_read: usize = socket.read(&mut buffer).await.unwrap();
    if bytes_read < 16384 {
        complete_buffer.extend_from_slice(&buffer[..bytes_read]);
        break;
    }
    complete_buffer.extend_from_slice(&buffer);
}
```
+ Responsive UI: By using DaisyUI, the server has a responsive UI that adapts to the screen size, making it easier to use on mobile devices. The UI is very simple and has only the necessary elements to interact with the server.
+ Authentication: The server uses a token-based authentication system, where the user sends the email and password to the server and the server returns a token that the user must use in the requests that require authentication.

## Future Improvements and Bugs
+ Profile Picture: The server does not have a profile picture system, the user can only post images in the posts.
+ Multipart Requests: The server does not handle the multipart requests correctly from all sources, some requests do not have the end boundary and the server does not handle this situation, thus not uploading the image:
The following is an example of a multipart request that the server does not handle correctly:
```http
POST /api/upload HTTP/1.1
Host: 192.168.1.9:8080
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8
Accept-Encoding: gzip, deflate
Accept-Language: en-GB,en;q=0.9
Content-Type: multipart/form-data; boundary=----WebKitFormBoundaryiboQcJBikicvmXO6
Origin: http://192.168.1.9:8080
User-Agent: Mozilla/5.0 (iPhone; CPU iPhone OS 17_4_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4.1 Mobile/15E148 Safari/604.1
Connection: keep-alive
Upgrade-Insecure-Requests: 1
Referer: http://192.168.1.9:8080/
Content-Length: 2294268
Cookie: token=deb14cbe1d076171852c29d7f2657cff2571a767d8913d207676c839bc876950

------WebKitFormBoundaryiboQcJBikicvmXO6
Content-Disposition: form-data; name="title"

Title
------WebKitFormBoundaryiboQcJBikicvmXO6
Content-Disposition: form-data; name="content"

Content
------WebKitFormBoundaryiboQcJBikicvmXO6
Content-Disposition: form-data; name="image"; filename="IMG_4830.jpeg"
Content-Type: image/jpeg

......JFIF.....,.,..AMPF..*.Exif..MM.*...................................................................(...........1...........2...........<.......................i............	.Apple.iPhone 15 Plus.....H.......H....17.4.1..2024:05:15 22:37:41.iPhone 15 Plus...$........................."...........'..................0232.................................
......

.....
```
The following is an example of a multipart request that the server handles correctly:
```http
POST /api/upload HTTP/1.1
Host: 127.0.0.1:8080
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:126.0) Gecko/20100101 Firefox/126.0
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8
Accept-Language: en-US,en;q=0.5
Accept-Encoding: gzip, deflate, br, zstd
Content-Type: multipart/form-data; boundary=---------------------------170211189117253210923498279470
Content-Length: 2750913
Origin: http://127.0.0.1:8080
Connection: keep-alive
Referer: http://127.0.0.1:8080/
Cookie: token=0462a891efd9c40cb53749f31da73c79dd66bea6d3a505b6c26a440b1c33cb8b
Upgrade-Insecure-Requests: 1
Sec-Fetch-Dest: document
Sec-Fetch-Mode: navigate
Sec-Fetch-Site: same-origin
Sec-Fetch-User: ?1
Priority: u=1

-----------------------------170211189117253210923498279470
Content-Disposition: form-data; name="title"

Title
-----------------------------170211189117253210923498279470
Content-Disposition: form-data; name="content"

Content
-----------------------------170211189117253210923498279470
Content-Disposition: form-data; name="image"; filename="CatppuccinMocha-Kurzgesagt-StellarPhenomenon.png"
Content-Type: image/png

.PNG
.
...
IHDR.......p.......\%.)..IDATx.....$I.$I.....GDDfffVUUUUwwwww......................................................................................twwwwWWUUUUffFFD....
.LfWwuwwO.....Lb....r.UW]././... ..:I`.I@)..g.T@. ..."..J.m...t
...(t....ny.-.yx7[...J.....R.6.<."._.F.v3.M#...q.<...g<.	.ptt.......I.
8S..O%	..U..I.lK...
."..w........K...9......N..4.n|.....r?..-@.-...."II.c}.o.......Zk..,.q....f...J-...n...'O\....M.l..df.T.9.)..2y~.t.kd.x~""[.R23s..i].[....w............. 3#..D......H.?.m@.`..L.....2....y..."
.Z...	...D..O..g......_G<7. ..<.x6.l...{	.. ....i..!@
.........i....e..e6..6`[Rf..-3...8..M.z=N.8..M...m..xN.y...L.0.@<.A<..?......A`.....	...$..$.<.y...../....m,.m.,.m.,.m.,.m.,.m.,.m.,.m...H..`..I...F.<.-		#i..x..{.k..../.J...a'...........Ia'..M.Pj.~m.xN..6...]......g.....B..I.k..~......$.6 	...&	...$q?.<?...m.'..n..I...
HB.2gr?I..I.e....x....>V.(....l...M....jy4........ry.\.9[fr.m....&...l./Q.....|..gg.........|..*.F.."E....BR.PJ.Z..R*....2..Q...D..%Q*.....4.m.<:s.
o.......g....xn.?...`R......2..LG@..vu.?...v.mO9y.....&..8.....8.xNR....$...
......).3.L....|c.<..1J....-pF.p.Q;.EN.E.......)$...q8:<X..Oz.?.q...i2.Eg...6.z...m@..*....0.2!.....H...|.W~.......QJNc...	..?H.z`..R;.%....$.v.f.g3..h..`G..."..H..i.J..`..Q*/X.F.5.4..J..4..v\....8.l.4.WGO.....o.|.....x6......_.6...l..x...$.....E|.......f.jd.x.....o....}....y.;y&.....;..m..3	..B......._O.`...D....M7=.......C.B!I<@D.2;..Y...#..lG.R.....=...(Y........?.I. )3%.".d..d...I.m@..IRD....n~.Cg.yDd&...^..._...K$..!3.I..6...."..If...D.4M..c.O..{}`.1dX.egp...........!c.d.0...S......S.a....`e.W.......7..Z.E)......
lK....$..I.l._.6....T..."..$..
...{..j....ls..&..$ "6.v.w...Rj(x...vk.mI<..._A`.....l.m.3`.HH.._%).......J.].u]....
d&..R..e.....$..l._..$....J).5..y....W~.7..%^A3. K..B"S!..s?....\2..?.A....6...?i..g....@N.\...4...v=..R;.S........{....72......./2.M.g1...xn2..6.........tA.i\.~...4.n....].8..M...3...c...........QZ&..K..^...c.o{.....v...L@...q&.V........e..~..~..~.....g....@.m.....l.2..l..y..........y6...<.....*W._'...i-..R.i.g3I\.Z.w...M......Z."b.&...""3...Z..1.2g*......MSD.Zk.m...,...e..I...6`I..`....>g..R.lK...V.....,J....F.lMR.....Ef[... .. ....Iv."byt....)a^4J\..(......T...R......l...*@..U.W8..(..QM*".-.G6....A...t...r.%.<...*.F.............cD4...IiG;.]8.........`........K.5g:33.i..R....$IB... "....&..v...".d.$ ....i.......[RJ..RJ...P./3..........)...H.2..H.L.!......2.H.YvJR.....(.r...P.l.....o.&Et].Rl......0...N..".....R.vf:.$..R+.-.6W..c[.m.NT....._..3.I\.?.mI.+.....mc"J)..j... .#....&.6..lK.$..?.$..1.I...k..''.PR.	`..#0.....d...0.x.H...'(.E.f.pf.f8.......$.....U.r.j...nX.a%	h..0W.[L.P..........R........../..t.".>..~..?.3....*..~..~.....3...@.\f....	..a.........._.?..~.4........IEND.B`.
-----------------------------170211189117253210923498279470--

HTTP/1.1 301 MOVED PERMANENTLY
Location: /
```
The code thus needs to be modified to handle the image data in the request body.

## License
Currently, this code is under the GNU AFFERO GENERAL PUBLIC LICENSE Version 3, 19 November 2007.
Rights remain with the original authors of the code.

## Authors
The original authors of the code are:
- [x] zpx (the owner of the repository)
- [x] redux for helping with the sql intelliphense
- [x] fba06 for testing with WebKit Apple Products that are often a pain to work with
- [x] midee for covering me during lessons so I could work on this >:D
- [x] zpx's cat for testing the application
![cat](cat.jpeg)
