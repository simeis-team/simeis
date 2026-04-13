// window.onload = function() {
//   // <editor-fold desc="Changeable Configuration Block">

//   // the following lines will be replaced by docker/configurator, when it runs in a docker-container
//   window.ui = SwaggerUIBundle({
//     url: "https://petstore.swagger.io/v2/swagger.json",
//     dom_id: '#swagger-ui',
//     deepLinking: false,
//     presets: [
//       // SwaggerUIBundle.presets.apis,
//       // SwaggerUIStandalonePreset
//     ],
//     plugins: [
//       // SwaggerUIBundle.plugins.DownloadUrl
//     ],
//     layout: "StandaloneLayout"
//   });

//   //</editor-fold>
// };

window.onload = function () {
  const ui = SwaggerUIBundle({
    url: "/swagger.json", // or your OpenAPI spec URL
    dom_id: "#swagger-ui",

    presets: [
      SwaggerUIBundle.presets.apis
    ],

    layout: "BaseLayout", // minimal layout

    deepLinking: false,
    docExpansion: "none",
    defaultModelsExpandDepth: -1, // hides schemas section
    defaultModelExpandDepth: -1,

    showExtensions: false,
    showCommonExtensions: false,

    tryItOutEnabled: true,

    // Disable top bar completely
    plugins: [
      SwaggerUIBundle.plugins.DownloadUrl
    ]
  });

  window.ui = ui;
};
