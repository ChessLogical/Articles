

<h1>Articles</h1>




<h3>Overview</h3>

Articles is a web application that allows users to create, view, and delete posts. Each post can include a title, message, and optionally a media file. The application uses a hybrid approach with both static HTML files and a Sled database for storing metadata. The user interface is designed to be clean and user-friendly, with a focus on performance and reliability.

<h3>Features</h3>
The application offers the following features: Users can create posts with a title, message, and optionally an image, video, or audio file. Posts are displayed on the main page, with links to view each post in detail. Users have the option to delete their posts within 2 minutes of creation. Admins can manage posts, including editing and deleting any post at any time.

<h3>Technology Stack</h3>
The core logic of the application is written in Rust using the Rocket framework. A high-performance embedded database called Sled is used for storing metadata about posts. Nginx is used as a web server to serve static files and proxy requests to the Rocket application. The front-end interface is built with HTML and CSS for a clean and responsive design.

<h3>Installation</h3>
To install and run the application, first clone the repository by running git clone https://github.com/ChessLogical/Articles.git. Navigate to the project directory with cd adelia. Ensure you have Rust installed, which you can do by following instructions at rust-lang.org. Install the necessary Rust dependencies by running cargo build. Set up Nginx to serve static files and proxy requests to the Rocket application. An example Nginx configuration is provided in the repository. Finally, start the application using cargo run.

<h3>Configuration</h3>
Ensure that the Sled database path is correctly set in the application. Additionally, make sure Nginx is configured to serve index.html files by default and proxy other requests to the Rocket application.

<h3>Usage</h3>
Once the application is running, you can access it via your web browser. The main page is located at https://4chess.com, (will be on and off while in dev-- if its not up and working wait a bit and try again) where you can view all posts and create new ones. To create a post, click the "Create New Post" button on the main page, fill in the title, message, and optionally upload a media file, then submit the form to create the post. To view a post, click on any post title to view the full post. Posts can be deleted within 2 minutes of creation by clicking the "Delete" button. The admin panel can be accessed by navigating to the admin URL configured in the application, where you can log in using the admin password to manage posts.

<h3>Customization</h3>
You can customize various aspects of the application. The HTML templates are located in the templates directory and can be modified to change the look and feel of the application. CSS styles are embedded in the HTML templates and can be adjusted to match your desired design. You can also modify the Rocket configuration and Nginx settings as needed to fit your environment.

<h3>Security</h3>
For security, ensure that the admin password and admin URL are properly configured and kept confidential. Regularly update dependencies and monitor for security vulnerabilities.

<h3>Contributing</h3>
Contributions are welcome! Please fork the repository and submit pull requests for any improvements or bug fixes.

<h3>License</h3>
This project is licensed under the MIT License. See the LICENSE file for details.

<h3>Support</h3>
If you encounter any issues or have questions, please open an issue on the GitHub repository.

<h3>Understanding Sled</h3>
What is Sled?
Sled is a modern, high-performance, embedded key-value store written in Rust. It is designed to be used as a local database for applications that require fast, reliable storage. Sled aims to provide a simple interface while ensuring durability and performance. It supports complex operations like transactions, and offers a robust foundation for building data-driven applications.

How Sled Works in This Application
In the 4Chess Message Board application, Sled is used to store metadata about posts. This includes the post title, message, directory path, media file path (if any), and the timestamp of the post. Here’s a detailed breakdown of how Sled is integrated and used in the application:

Database Initialization: When the application starts, a Sled database is opened or created. This is done using the sled::open function, which returns a handle to the database. This handle is stored in the application state for easy access.

Storing Posts: When a new post is created, its metadata is serialized into JSON and stored in the Sled database. The key for each post is derived from a unique identifier generated for the post. The value is the serialized JSON representation of the post's metadata.

Retrieving Posts: To display posts on the main page, the application retrieves all entries from the Sled database, deserializes them, and formats them for display. This allows for fast and efficient access to post metadata without having to read from individual files.

Deleting Posts: When a post is deleted, its entry is removed from the Sled database. The corresponding static HTML file and media files are also deleted to keep the file system clean.

Updating Posts: Admins can update post metadata (such as the title or message) through the admin panel. The updated metadata is written back to the Sled database, ensuring that the stored data remains consistent.

Hybrid Approach: While Sled handles metadata storage, the actual post content (HTML files) is stored as static files on the server. This hybrid approach leverages the strengths of both static files and a fast key-value store, ensuring that post retrieval is quick and efficient while also providing the simplicity of serving static HTML content.

Why Use Sled?
Sled was chosen for this application due to its high performance, reliability, and ease of use. It provides a simple yet powerful API for storing and retrieving data, making it an excellent choice for managing the metadata of posts in a web application. Additionally, being written in Rust ensures that it benefits from Rust’s safety guarantees and performance optimizations.

